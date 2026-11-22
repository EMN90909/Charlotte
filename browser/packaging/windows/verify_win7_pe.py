#!/usr/bin/env python3
"""
Charlotte Windows 7 SP1 PE Binary Import Auditor.

Inspects actual Windows PE32 / PE32+ executable & DLL import tables (using pure Python stdlib)
to verify that compiled Win7 ESR binaries do NOT contain hard imports from Windows 8/10/11-only
DLLs or APIs (e.g., WaitOnAddress, ProcessPrng, GetSystemTimePreciseAsFileTime,
api-ms-win-core-synch-l1-2-0.dll, bcryptprimitives.dll, api-ms-win-core-winrt-*).

Usage:
  python3 verify_win7_pe.py path/to/Charlotte.exe [other.dll ...]
"""

import os
import struct
import sys
from typing import List, Tuple

FORBIDDEN_DLLS = {
    "api-ms-win-core-synch-l1-2-0.dll",
    "api-ms-win-core-winrt-l1-1-0.dll",
    "api-ms-win-core-winrt-string-l1-1-0.dll",
    "api-ms-win-shcore-scaling-l1-1-1.dll",
    "bcryptprimitives.dll",
    "combase.dll",
}

FORBIDDEN_APIS = {
    "waitonaddress",
    "wakebyaddresssingle",
    "wakebyaddressall",
    "getsystemtimepreciseasfiletime",
    "processprng",
    "setthreaddescription",
    "getdpiforwindow",
    "setprocessdpiawarenesscontext",
    "roinitialize",
    "rogetactivationfactory",
    "createfile2",
    "discardvirtualmemory",
}


def rva_to_offset(rva: int, sections: List[Tuple[int, int, int, int]]) -> int:
    for vaddr, vsize, raw_ptr, raw_size in sections:
        span = max(vsize, raw_size)
        if vaddr <= rva < vaddr + span:
            return raw_ptr + (rva - vaddr)
    return -1


def read_cstr(data: bytes, offset: int) -> str:
    if offset < 0 or offset >= len(data):
        return ""
    end = data.find(b"\x00", offset)
    if end == -1:
        end = len(data)
    return data[offset:end].decode("ascii", errors="ignore")


def parse_pe_imports(path: str) -> Tuple[List[str], List[str]]:
    with open(path, "rb") as f:
        data = f.read()

    if len(data) < 0x40 or data[:2] != b"MZ":
        raise ValueError(f"{path}: Not a valid PE (missing MZ header)")

    pe_offset = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe_offset : pe_offset + 4] != b"PE\x00\x00":
        raise ValueError(f"{path}: Missing PE signature")

    coff_off = pe_offset + 4
    _, num_sections, _, _, _, opt_hdr_size, _ = struct.unpack_from("<HHIIIHH", data, coff_off)
    opt_off = coff_off + 20
    magic = struct.unpack_from("<H", data, opt_off)[0]

    if magic == 0x10B:  # PE32 (32-bit x86)
        is_pe64 = False
        dd_off = opt_off + 96
    elif magic == 0x20B:  # PE32+ (64-bit x64)
        is_pe64 = True
        dd_off = opt_off + 112
    else:
        raise ValueError(f"{path}: Unknown PE optional header magic 0x{magic:x}")

    # DataDirectory[1] = Import Table
    import_rva, import_size = struct.unpack_from("<II", data, dd_off + 8)
    sec_off = opt_off + opt_hdr_size
    sections = []
    for i in range(num_sections):
        s = sec_off + i * 40
        vsize, vaddr, raw_size, raw_ptr = struct.unpack_from("<IIII", data, s + 8)
        sections.append((vaddr, vsize, raw_ptr, raw_size))

    dlls: List[str] = []
    apis: List[str] = []

    if import_rva == 0 or import_size == 0:
        return dlls, apis

    desc_off = rva_to_offset(import_rva, sections)
    if desc_off < 0:
        return dlls, apis

    while desc_off + 20 <= len(data):
        orig_first_thunk, _, _, name_rva, first_thunk = struct.unpack_from("<IIIII", data, desc_off)
        if orig_first_thunk == 0 and name_rva == 0 and first_thunk == 0:
            break
        dll_name = read_cstr(data, rva_to_offset(name_rva, sections))
        if dll_name:
            dlls.append(dll_name)

        thunk_rva = orig_first_thunk if orig_first_thunk != 0 else first_thunk
        thunk_off = rva_to_offset(thunk_rva, sections)
        if thunk_off >= 0:
            step = 8 if is_pe64 else 4
            ordinal_flag = 1 << 63 if is_pe64 else 1 << 31
            while thunk_off + step <= len(data):
                val = struct.unpack_from("<Q" if is_pe64 else "<I", data, thunk_off)[0]
                if val == 0:
                    break
                if (val & ordinal_flag) == 0:
                    hint_off = rva_to_offset(val & 0x7FFFFFFF, sections)
                    if hint_off >= 0:
                        fn_name = read_cstr(data, hint_off + 2)
                        if fn_name:
                            apis.append(fn_name)
                thunk_off += step
        desc_off += 20

    return dlls, apis


def audit_file(path: str) -> List[str]:
    dlls, apis = parse_pe_imports(path)
    violations = []
    for dll in dlls:
        if dll.lower() in FORBIDDEN_DLLS:
            violations.append(f"Forbidden Win8+/Win10+ DLL import: {dll}")
    for api in apis:
        if api.lower() in FORBIDDEN_APIS:
            violations.append(f"Forbidden Win8+/Win10+ API import: {api}")
    return violations


def main() -> int:
    if len(sys.argv) < 2:
        print("Usage: verify_win7_pe.py <pe-file-or-dir> [...]")
        return 2

    targets: List[str] = []
    for arg in sys.argv[1:]:
        if os.path.isdir(arg):
            for root, _, files in os.walk(arg):
                for fn in files:
                    if fn.lower().endswith((".exe", ".dll")):
                        targets.append(os.path.join(root, fn))
        elif os.path.isfile(arg):
            targets.append(arg)

    if not targets:
        print("No .exe or .dll binaries found to audit.")
        return 0

    failed = False
    for path in targets:
        try:
            violations = audit_file(path)
            if violations:
                failed = True
                print(f"[FAIL] {path}:")
                for v in violations:
                    print(f"  - {v}")
            else:
                print(f"[PASS] {path}: Win7 SP1 import table clean")
        except Exception as exc:
            failed = True
            print(f"[ERROR] {path}: {exc}")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
