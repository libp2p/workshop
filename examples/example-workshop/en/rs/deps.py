#!/usr/bin/env python3
import sys
import subprocess
import shutil

def check_command(command, min_version=None):
    try:
        # Check if command exists
        result = subprocess.run([command, "--version"], capture_output=True, text=True)
        if result.returncode != 0:
            print(f"! {command} is not installed")
            return False
        
        # If a minimum version is specified, check it (simplified version check)
        if min_version and min_version not in result.stdout:
            print(f"! {command} version {min_version} or higher is required")
            return False
        
        print(f"* {command} is installed")
        return True
    except FileNotFoundError:
        print(f"! {command} is not installed")
        return False

def main():
    # Check required dependencies
    all_dependencies_met = True
    
    # Check if rust is installed
    if not check_command("rustc"):
        all_dependencies_met = False
    
    # Check if cargo is installed
    if not check_command("cargo"):
        all_dependencies_met = False
    
    # Check if docker is installed
    if not check_command("docker"):
        all_dependencies_met = False
    
    # Check if docker-compose is installed
    docker_compose_path = shutil.which("docker-compose") or shutil.which("docker") 
    
    if docker_compose_path:
        # Check if docker compose plugin is available
        if docker_compose_path == shutil.which("docker"):
            result = subprocess.run(["docker", "compose", "version"], capture_output=True, text=True)
            if result.returncode != 0:
                print("! docker compose plugin is not installed")
                all_dependencies_met = False
            else:
                print("* docker compose is installed")
        else:
            print("* docker-compose is installed")
    else:
        print("! docker-compose is not installed")
        all_dependencies_met = False
    
    if all_dependencies_met:
        print("\nAll dependencies are met!")
        sys.exit(0)
    else:
        print("\nSome dependencies are missing. Please install them before continuing.")
        sys.exit(1)

if __name__ == "__main__":
    main()
