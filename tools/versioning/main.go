package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"regexp"

	toml "github.com/pelletier/go-toml/v2"
)

const webPackageRelPath = "web/package.json"

type cargoDoc struct {
	Workspace struct {
		Package struct {
			Version string `toml:"version"`
		} `toml:"package"`
	} `toml:"workspace"`
}

type packageDoc struct {
	Version string `json:"version"`
}

func parseWorkspaceVersion(cargoBytes []byte) (string, error) {
	var doc cargoDoc
	if err := toml.Unmarshal(cargoBytes, &doc); err != nil {
		return "", err
	}
	if doc.Workspace.Package.Version == "" {
		return "", fmt.Errorf("workspace.package.version not found")
	}
	return doc.Workspace.Package.Version, nil
}

func readWorkspaceVersion(cargoPath string) (string, error) {
	cargoBytes, err := os.ReadFile(cargoPath)
	if err != nil {
		return "", err
	}
	return parseWorkspaceVersion(cargoBytes)
}

func readPackageVersion(pkgPath string) (string, error) {
	b, err := os.ReadFile(pkgPath)
	if err != nil {
		return "", err
	}

	var pkg packageDoc
	if err := json.Unmarshal(b, &pkg); err != nil {
		return "", err
	}
	if pkg.Version == "" {
		return "", fmt.Errorf("version field missing in %s", pkgPath)
	}

	return pkg.Version, nil
}

func writePackageVersion(pkgPath, version string) error {
	b, err := os.ReadFile(pkgPath)
	if err != nil {
		return err
	}
	re := regexp.MustCompile(`("version"\s*:\s*)"[^"]*"`)
	updated := re.ReplaceAll(b, []byte(`${1}"`+version+`"`))
	if len(updated) == len(b) && string(updated) == string(b) {
		return nil
	}
	return os.WriteFile(pkgPath, updated, 0644)
}

func findRepoRoot(start string) (string, error) {
	dir := start
	for {
		cargoPath := filepath.Join(dir, "Cargo.toml")
		if st, err := os.Stat(cargoPath); err == nil && !st.IsDir() {
			return dir, nil
		}

		parent := filepath.Dir(dir)
		if parent == dir {
			break
		}
		dir = parent
	}
	return "", errors.New("could not find Cargo.toml in current or parent directories")
}

func run() error {
	check := flag.Bool("check", false, "check versions without writing files")
	printOnly := flag.Bool("print", false, "print cargo and web versions")
	flag.Parse()

	if *check && *printOnly {
		return fmt.Errorf("-check and -print cannot be used together")
	}

	cwd, err := os.Getwd()
	if err != nil {
		return err
	}

	repoRoot, err := findRepoRoot(cwd)
	if err != nil {
		return err
	}

	cargoPath := filepath.Join(repoRoot, "Cargo.toml")
	webPath := filepath.Join(repoRoot, webPackageRelPath)

	cargoVersion, err := readWorkspaceVersion(cargoPath)
	if err != nil {
		return err
	}
	webVersion, err := readPackageVersion(webPath)
	if err != nil {
		return err
	}

	if *printOnly {
		fmt.Printf("cargo=%s\n", cargoVersion)
		fmt.Printf("web=%s\n", webVersion)
		return nil
	}

	if *check {
		if webVersion != cargoVersion {
			return fmt.Errorf("version drift detected: %s=%s, Cargo.toml=%s", webPackageRelPath, webVersion, cargoVersion)
		}
		fmt.Println("Versions are in sync:", cargoVersion)
		return nil
	}

	if webVersion == cargoVersion {
		fmt.Println("web/package.json already in sync:", cargoVersion)
		return nil
	}

	if err := writePackageVersion(webPath, cargoVersion); err != nil {
		return err
	}
	fmt.Printf("Synced %s: %s -> %s\n", webPackageRelPath, webVersion, cargoVersion)
	return nil
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
