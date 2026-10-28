package supabaseauth

import (
  "encoding/json"
  "net/http"
  "net/http/httptest"
  "testing"
)

func TestMeHandler(t *testing.T) {
  upstream := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
    if r.URL.Path != "/auth/v1/user" || r.Header.Get("Authorization") != "Bearer valid-token" ||
      r.Header.Get("apikey") != "sb_publishable_test" {
      http.Error(w, "not authorized", http.StatusUnauthorized)
      return
    }
    _ = json.NewEncoder(w).Encode(User{ID: "user-123", Email: "person@example.com"})
  }))
  defer upstream.Close()
  h := MeHandler(upstream.URL, "sb_publishable_test")

  unauth := httptest.NewRecorder()
  h.ServeHTTP(unauth, httptest.NewRequest("GET", "/auth/me", nil))
  if unauth.Code != http.StatusUnauthorized { t.Fatalf("missing bearer: %d", unauth.Code) }

  req := httptest.NewRequest("GET", "/auth/me", nil)
  req.Header.Set("Authorization", "Bearer valid-token")
  success := httptest.NewRecorder()
  h.ServeHTTP(success, req)
  if success.Code != http.StatusOK { t.Fatalf("valid token: %d body=%s", success.Code, success.Body.String()) }
  var user User
  if err := json.Unmarshal(success.Body.Bytes(), &user); err != nil || user.ID != "user-123" {
    t.Fatalf("unexpected user response: %#v, err %v", user, err)
  }

  wrong := httptest.NewRequest("GET", "/auth/me", nil)
  wrong.Header.Set("Authorization", "Bearer wrong")
  denied := httptest.NewRecorder()
  h.ServeHTTP(denied, wrong)
  if denied.Code != http.StatusUnauthorized { t.Fatalf("invalid token: %d", denied.Code) }
}

func TestMeHandlerUnconfigured(t *testing.T) {
  res := httptest.NewRecorder()
  MeHandler("", "").ServeHTTP(res, httptest.NewRequest("GET", "/auth/me", nil))
  if res.Code != http.StatusServiceUnavailable { t.Fatalf("expected 503 got %d", res.Code) }
}
