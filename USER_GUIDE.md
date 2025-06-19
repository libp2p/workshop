# Workshop User Guide

The `workshop` program is a command-line tool for running programming workshops. This application provides a Terminal User Interface (TUI) for browsing and completing workshops with interactive lessons.

## Prerequisites

The `workshop` application requires a terminal that supports ANSI escape codes. This is typically any modern terminal on Linux, macOS, or Windows. The application also requires Docker to be installed on your system, as it uses Docker to run the lessons in isolated environments. You must have Git installed on your system if you wish to install workshops from a repository URL. Lastly, you must have Python 3.10 or newer installed on your system, as the `workshop` applicataion relies upon Python scripts for checking solutions.

## Quick Start

Running the `workshop` program initially presents you with the workshop selection screen that lists all of the workshops available to you. Currently the workshops are stored in the `/home/<username>/.local/share/workshop` folder on Linux, in `/Users/<username>/Library/Application Support/io.libp2p.workshop` folder on macOS, and in the `C:\Users\<username>\AppData\Roaming\io\libp2p\workshop\data` folder on Windows. Adding more workshops is done by running `workshop --install <repo-url>` where `<repo-url>` is the URL of the workshop repository. The `workshop` application will clone the repository into the appropriate folder and make it available in the workshop selection screen.

In the workshop selection screen, you can navigate through the list of available workshops using the arrow keys. The right side of the UI displays the information about the currently highlighted workshop. This includes the title, author, copyright, license, and a description of the workshop. The description also includes the setup instructions for the workshop. This typically include creatin a new project folder for your chosen programming language or cloning a repository. Once you have followed the setup instructions, exit the `workshop` application, change into the project directory and re-run the `workshop` program. This will again take you to the workshop selection screen. By pressing the `Enter` key, you can select a workshop and start working through the lessons in the project folder.

After selecting a workshop, the `workshop` application will run a script to check if you have all of the required tools installed on your system. If you do not have the required tools installed, the application will provide instructions on how to install them. Once the required tools are installed, the application presents the lesson selection screen. Here you select your lesson. Lessons are designed so that you must complete the previous lessons before you can complete the next lesson. This is to ensure that you have the required knowledge to complete the next lesson. The lesson selection screen displays the title and description of each lesson. You can navigate through the list of lessons using the arrow keys and press the `Enter` key to select a lesson.

When you select a lesson, the application shows the lesson to you. This is a scrollable markdown viewer that presents the lesson content. The lesson very likely has hints through the lesson. Each hint starts off collapsed so as to not spoil the challenge of learning. If you get stuck and really need a hint, scroll down until the hint is highlighted and press enter to expand it.

When you believe you have completed the lesson, hit the `c` key to check your solution. The `workshop` application runs a script to build a Docker image from your solution, runs it, and checks the output against the expected output. Sometimes checking your solution requires setting up multiple running Docker images that commicate with each other over the network. These details are hidden from you.

If your solution successfully passes the check, the application displays a success message and goes back to the lesson selection screen. If your solution fails the check, the application displays the error message and you can try again. Once completed, a lesson is marked as complete. At any time you may go back and re-read a lesson and the hints.

## Typical Flow

1. Run the `workshop` program in your terminal and read the setup instructions for the workshop you want to complete.
2. Complete the setup instructions in another terminal. This typically involves creating a new project folder or cloning a repository.
3. Exit the `workshop` program and change into the project directory.
4. Run the `workshop` program again in your terminal and select the workshop.
5. Select your programming language.
6. Select the lesson you want to complete. 
7. Complete the lesson by following the instructions in the lesson.
8. If you get stuck, scroll down to the hint and press enter to expand it.
9. When you believe you have completed the lesson, hit the `c` key to check your solution.
10. Complete all lessons.

## Multiple Workshops in a Series

The `workshop` tool is designed to support completing multiple workshops in a series that build on the previous workshop, all in the same project folder. Once you have completed a workshop, you hit the `b` key to go back to the workshop selection screen. From there you can select next workshop in the series. The `workshop` application will run a script to check if you have all of the required tools installed on your system as well as check if you have completed the previous workshop(s). 
