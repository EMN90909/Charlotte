package main

import "testing"

func TestConfiguredHTTPAddress(t *testing.T) {
	tests := []struct {
		name, appPort, httpPort, expected string
		wantErr bool
	}{
		{"default", "", "", ":8080", false},
		{"platform", "10000", "", ":10000", false},
		{"explicit override", "10000", "9000", ":9000", false},
		{"invalid port", "unknown", "", "", true},
		{"negative", "", "-1", "", true},
		{"outside range", "", "65536", "", true},
	}
	for _, testCase := range tests {
		t.Run(testCase.name, func(t *testing.T) {
			t.Setenv("PORT", testCase.appPort)
			t.Setenv("HTTP_PORT", testCase.httpPort)
			actual, err := configuredHTTPAddress()
			if (err != nil) != testCase.wantErr || actual != testCase.expected {
				t.Fatalf("got %q, %v; expected %q, err=%t", actual, err, testCase.expected, testCase.wantErr)
			}
		})
	}
}
