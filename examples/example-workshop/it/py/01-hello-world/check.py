#!/usr/bin/env python3
import sys
import os

def check_stdout_log():
    log_path = "app/stdout.log"
    
    # Check if the log file exists
    if not os.path.exists(log_path):
        print("❌ Error: stdout.log file not found.")
        return False
        
    # Read the log file
    with open(log_path, 'r') as f:
        content = f.read().strip()
    
    # Check if it contains "Hello, world!"
    if "Hello, world!" in content:
        print("✅ Success! Your program correctly outputs 'Hello, world!'")
        return True
    else:
        print(f"❌ Your program output '{content}' but we expected 'Hello, world!'")
        return False

def main():
    print("Checking your solution...")
    
    if check_stdout_log():
        print("\nGreat job! You've successfully completed the 'Hello, World!' lesson.")
        sys.exit(0)
    else:
        print("\nPlease check your code and try again.")
        sys.exit(1)

if __name__ == "__main__":
    main()
