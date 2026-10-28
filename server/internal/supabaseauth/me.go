// Package supabaseauth verifies Charlotte account access tokens against
// Supabase Auth's /auth/v1/user endpoint. It never accepts an unverified JWT
// payload, and no service-role key is required or accepted here.
package supabaseauth

import (
  "encoding/json"
  "net/http"
  "net/url"
  "os"
  "strings"
  "time"
)

type User struct {
  ID string `json:"id"`
  Email string `json:"email"`
}

var client = &http.Client{Timeout: 7 * time.Second}

func MeHandlerFromEnv() http.Handler {
  return MeHandler(os.Getenv("SUPABASE_URL"), os.Getenv("SUPABASE_PUBLISHABLE_KEY"))
}

// MeHandler is a read-only identity validation endpoint. Accepts only a
// Supabase user access token; never an internal chat JWT.
func MeHandler(base, publishable string) http.Handler {
  u, err := url.Parse(strings.TrimSpace(base))
  valid := err == nil && u != nil && u.Host != "" &&
    (u.Scheme == "https" || (u.Scheme == "http" && (u.Hostname() == "localhost" || u.Hostname() == "127.0.0.1")))
  return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
    w.Header().Set("Cache-Control", "no-store")
    if !valid || publishable == "" || strings.Contains(publishable, "service_role") {
      http.Error(w, "Supabase Auth is not configured", http.StatusServiceUnavailable)
      return
    }
    header := r.Header.Get("Authorization")
    if !strings.HasPrefix(header, "Bearer ") || len(strings.TrimSpace(strings.TrimPrefix(header, "Bearer "))) == 0 {
      http.Error(w, "Bearer access token required", http.StatusUnauthorized)
      return
    }
    endpoint := *u
    endpoint.Path = strings.TrimRight(endpoint.Path, "/") + "/auth/v1/user"
    endpoint.RawQuery = ""
    request, err := http.NewRequestWithContext(r.Context(), http.MethodGet, endpoint.String(), nil)
    if err != nil { http.Error(w, "Request failed", http.StatusBadGateway); return }
    request.Header.Set("apikey", publishable)
    request.Header.Set("Authorization", header)
    response, err := client.Do(request)
    if err != nil { http.Error(w, "Identity provider unavailable", http.StatusBadGateway); return }
    defer response.Body.Close()
    if response.StatusCode != http.StatusOK {
      http.Error(w, "Invalid or expired access token", http.StatusUnauthorized)
      return
    }
    var user User
    if err = json.NewDecoder(http.MaxBytesReader(w, response.Body, 16384)).Decode(&user); err != nil || user.ID == "" {
      http.Error(w, "Invalid identity response", http.StatusBadGateway)
      return
    }
    w.Header().Set("Content-Type", "application/json")
    _ = json.NewEncoder(w).Encode(user)
  })
}
