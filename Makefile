# Powershell Default Makefile
SHELL := pwsh.exe
.SHELLFLAGS := -NoProfile -Command

.PHONY: release release-dirty release-dry release-headless release-skip-ci version \
        build build-release build-version \
        build-all \
        build-all-windows-amd64 build-all-windows-arm64 \
        build-all-linux-amd64   build-all-linux-arm64   \
        build-all-darwin-amd64  build-all-darwin-arm64  \
        clean

# run (no persistent binary)
release:
	Push-Location tools/release; go run .; Pop-Location

release-dirty:
	Push-Location tools/release; go run . --dirty; Pop-Location

release-dry:
	Push-Location tools/release; go run . --dry-run; Pop-Location

release-headless:
	Push-Location tools/release; go run . --headless; Pop-Location

release-skip-ci:
	Push-Location tools/release; go run . --skip-ci; Pop-Location

version:
	Push-Location tools/versioning; go run .; Pop-Location

# build binaries into tools/builds/
build: build-release build-version

build-release:
	Push-Location tools/release; go build -o ../builds/release.exe .; Pop-Location

build-version:
	Push-Location tools/versioning; go build -o ../builds/version.exe .; Pop-Location

# cross-platform builds → tools/builds/<os>-<arch>/
build-all: \
	build-all-windows-amd64 build-all-windows-arm64 \
	build-all-linux-amd64   build-all-linux-arm64   \
	build-all-darwin-amd64  build-all-darwin-arm64

build-all-windows-amd64:
	Push-Location tools/release;    $$env:GOOS='windows'; $$env:GOARCH='amd64'; go build -o ../builds/windows-amd64/release.exe .; Pop-Location
	Push-Location tools/versioning; $$env:GOOS='windows'; $$env:GOARCH='amd64'; go build -o ../builds/windows-amd64/version.exe .; Pop-Location

build-all-windows-arm64:
	Push-Location tools/release;    $$env:GOOS='windows'; $$env:GOARCH='arm64'; go build -o ../builds/windows-arm64/release.exe .; Pop-Location
	Push-Location tools/versioning; $$env:GOOS='windows'; $$env:GOARCH='arm64'; go build -o ../builds/windows-arm64/version.exe .; Pop-Location

build-all-linux-amd64:
	Push-Location tools/release;    $$env:GOOS='linux'; $$env:GOARCH='amd64'; go build -o ../builds/linux-amd64/release .; Pop-Location
	Push-Location tools/versioning; $$env:GOOS='linux'; $$env:GOARCH='amd64'; go build -o ../builds/linux-amd64/version .; Pop-Location

build-all-linux-arm64:
	Push-Location tools/release;    $$env:GOOS='linux'; $$env:GOARCH='arm64'; go build -o ../builds/linux-arm64/release .; Pop-Location
	Push-Location tools/versioning; $$env:GOOS='linux'; $$env:GOARCH='arm64'; go build -o ../builds/linux-arm64/version .; Pop-Location

build-all-darwin-amd64:
	Push-Location tools/release;    $$env:GOOS='darwin'; $$env:GOARCH='amd64'; go build -o ../builds/darwin-amd64/release .; Pop-Location
	Push-Location tools/versioning; $$env:GOOS='darwin'; $$env:GOARCH='amd64'; go build -o ../builds/darwin-amd64/version .; Pop-Location

build-all-darwin-arm64:
	Push-Location tools/release;    $$env:GOOS='darwin'; $$env:GOARCH='arm64'; go build -o ../builds/darwin-arm64/release .; Pop-Location
	Push-Location tools/versioning; $$env:GOOS='darwin'; $$env:GOARCH='arm64'; go build -o ../builds/darwin-arm64/version .; Pop-Location

clean:
	if (Test-Path tools/builds) { Remove-Item -Recurse -Force tools/builds }
