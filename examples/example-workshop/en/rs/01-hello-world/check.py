#!/usr/bin/env python3
import sys
import os

def check_stdout_log():
    log_path = "stdout.log"
    
    # Check if the log file exists
    if not os.path.exists(log_path):
        print("x Error: stdout.log file not found.")
        return False
        
    # Read the log file
    with open(log_path, 'r') as f:
        content = f.read().strip()
    
    # Check if it contains "Hello, world!"
    if "Hello, World!" in content:
        print("v Success! Your program works correctly.")
        return True
    else:
        print(f"x Your program output '{content}' but we expected 'Hello, World!'")
        return False

def main():
    print("r Checking your solution...")
    
    if check_stdout_log():
        print("Great job! You've successfully completed the 'Hello, World!' lesson.")
        sys.exit(0)
    else:
        print("Please check your code and try again.")
        sys.exit(1)

if __name__ == "__main__":
    main()
