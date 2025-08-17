#!/usr/bin/env python3
import sys
import subprocess
import re


def check_command(command, min_major: int | None = None):
    try:
        result = subprocess.run([command, "--version"], capture_output=True, text=True)
        if result.returncode != 0:
            print(f"! {command} is not installed")
            return False

        if min_major is not None:
            # Extract first integer in version output as major (e.g., v22.18.0 -> 22)
            out = result.stdout.strip() or result.stderr.strip()
            m = re.search(r"(\d+)", out)
            major = int(m.group(1)) if m else None
            if major is None or major < min_major:
                print(f"! {command} version {min_major}+ is required")
                return False

        print(f"v {command} is installed")
        return True
    except FileNotFoundError:
        print(f"! {command} is not installed")
        return False


def main():
    all_dependencies_met = True

    # Require Node.js 18+
    if not check_command("node", min_major=18):
        all_dependencies_met = False

    # npm should also be available
    if not check_command("npm"):
        all_dependencies_met = False

    if all_dependencies_met:
        print("All dependencies are met!")
        sys.exit(0)
    else:
        print("Some dependencies are missing. Please install them before continuing.")
        sys.exit(1)


if __name__ == "__main__":
    main()


