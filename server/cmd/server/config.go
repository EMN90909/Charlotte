package main

import (
	"fmt"
	"os"
	"strconv"
	"strings"
)

// configuredHTTPAddress lets one native binary run locally, in containers,
// and on PaaS hosts assigning a PORT. HTTP_PORT takes precedence over PORT.
// For backwards compatibility, the default remains :8080.
func configuredHTTPAddress() (string, error) {
	port := strings.TrimSpace(os.Getenv("HTTP_PORT"))
	if port == "" {
		port = strings.TrimSpace(os.Getenv("PORT"))
	}
	if port == "" {
		return httpAddr, nil
	}
	parsed, err := strconv.Atoi(port)
	if err != nil || parsed < 1 || parsed > 65535 {
		return "", fmt.Errorf("HTTP_PORT or PORT must be an integer from 1 to 65535")
	}
	return fmt.Sprintf(":%d", parsed), nil
}
