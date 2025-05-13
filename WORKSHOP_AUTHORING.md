# Workshop Creators Guide

This guide explains how to create workshops for the `workshop` application. The application is designed to provide a structured and interactive learning experience through a series of lessons.


## Workshop File Structure

Workshops are structured so that they can be completed in multiple programming languages and spoken languages. The file structure directly reflects this. In the following file structure diagram you see that the under the workshop root directory there are directories for each spoken language and under each spoken language directory there are directories for different programming languages. Each spoken language directory contains the lessons for different programming languages in that spoken language.

To use the `workshop` tool, the user must first either create a new folder for a project in their chosen programming language or use `git` to clone a repository. They then switch into the project directory and run the `workshop` tool. Initially they will choose their programming language and spoken language. Then they will choose which workshop they wish to complete. Once they have selected those three things, the `workshop` tool creates a folder in the project directory called `.workshops` and copied the chosed workshop folder into the `.workshops` folder. It initializes a `selected.yaml` file in the `.workshops` folder that identifies the workshop, the spoken language, and the programming language the user selected.

The structure of having a `selected.yaml` file in the root and `progress.yaml` files in the workshops allows the user to switch between multiple workshop in a given project folder in different spoken languages if they wish. The goal is to support multiple workshops in a single project folder that build upon each other. For instance you can image a workshop that teaches the basics of setting up a Rust projrect and writing a simple application. Then a second workshop that starts with the simple application and adds file I/O and error handling. Then a third that refactors the application into crates and a binary. Then a final workshop that builds on the third by refactoring the I/O to be asynchronous.

The following is an example of the file structure of a workshop. In this case the workshop has versions in English, Japanese and Hindi. The English version has lesson for Rust and Typescript. The Japanese version has lessons fo Python and Ruby. The Hindi version has lessons for Python and .Net.

```
workshop-name/                          # Workshop root directory
├── workshop.yaml                       # Workshop metadata
├── LICENSE                             # License text
│
├── en/                                 # English version of the workshop
│   ├── workshop.md                     # Workshop description in English
│   │
│   ├── rs/                             # Rust lessons in English
│   │   ├── setup.md                    # Setup instructions in English
│   │   ├── deps.py                     # Dependencies check script
│   │   │
│   │   ├── 1-lesson-name/              # First lesson
│   │   │   ├── lesson.yaml             # Lesson metadata
│   │   │   ├── lesson.md               # Lesson content with hints in English
│   │   │   ├── docker-compose.yaml     # Docker setup for testing
│   │   │   ├── check.py                # Solution build/run/check script
│   │   │   ├── tester1/                # First tester service
│   │   │   │   └── Dockerfile          # Dockerfile for the tester service
│   │   │   └── ...                     # Additional tester services
│   │   │
│   │   ├── 2-lesson-name/              # Second lesson
│   │   │   └── ...
│   │   │
│   │   └── ...                         # Additional lessons
│   │
│   ├── ts/                             # Typescript lessons in English
│   │   └── ...
│   │
│   └── ...                             # Other programming language lessons in English
│
├── jp/                                 # 日本語 version of the workshop
│   ├── workshop.md                     # Workshop description in 日本語
│   │
│   ├── py/                             # Python lessons in 日本語
│   │   ├── setup.md                    # Setup instructions in 日本語
│   │   ├── deps.py                     # Dependencies check script
│   │   │
│   │   ├── 1-lesson-name/              # First lesson
│   │   │   ├── lesson.yaml             # Lesson metadata 
│   │   │   ├── lesson.md               # Lesson content with hints in 日本語
│   │   │   ├── docker-compose.yaml     # Docker setup for testing
│   │   │   ├── check.py                # Solution build/run/check script
│   │   │   ├── tester1/                # First tester service
│   │   │   │   └── Dockerfile          # Dockerfile for the tester service
│   │   │   └── ...                     # Additional tester services
│   │   │
│   │   ├── 2-lesson-name/              # Second lesson
│   │   │   └── ...
│   │   │
│   │   └── ...                         # Additional lessons
│   │
│   ├── rb/                             # Ruby lessons in 日本語
│   │   └── ...
│   │
│   └── ...                             # Other programming language lessons in 日本語
│
├── hi/                                 # हिन्दीversion of the workshop
│   ├── workshop.md                     # Workshop description in हिन्दी
│   │
│   ├── py/                             # Python lessons in हिन्दी
│   │   ├── setup.md                    # Setup instructions in हिन्दी
│   │   ├── deps.py                     # Dependencies check script
│   │   │
│   │   ├── 1-lesson-name/              # First lesson
│   │   │   ├── lesson.yaml             # Lesson metadata
│   │   │   ├── lesson.md               # Lesson content with hints in हिन्दी
│   │   │   ├── docker-compose.yaml     # Docker setup for testing
│   │   │   ├── check.py                # Solution build/run/check script
│   │   │   ├── tester1/                # First tester service
│   │   │   │   └── Dockerfile          # Dockerfile for the tester service
│   │   │   └── ...                     # Additional tester services
│   │   │
│   │   ├── 2-lesson-name/              # Second lesson
│   │   │   └── ...
│   │   │
│   │   └── ...                         # Additional lessons
│   │
│   ├── cs/                             # .Net lessons in हिन्दी
│   │   └── ...
│   │
│   └── ...                             # Other programming language lessons in हिन्दी
│
└── ...                                 # Additional language versions of the workshop
```

The `workshop.yaml` file in the root directory contains metadata about the workshop, including the title, authors, copyright, license, homepage, and difficulty level. The `workshop.md` file under the different spoken language folders contains the description of the workshop in the spoken language. The `setup.md` files in the different programming languages have the setup instructions specific to the programming language in the respective spoken language for the workshop. This is where you tell your users how to set up a project folder or clone a repo to work in. Tell them to switch into the project folder and re-run the `workshop` application to continue. The `LICENSE` file contains the text of the license that governs the conten of the workshop.

Under each programming language folder, there is a `deps.py` Python script that gets executed when the workshop is selected. As a workshop author, you will need to implement this script to check that the required tools are properly installed on the user's system. The script should return a non-zero exit code if any of the required tools are not installed. The script should also print a message to the user indicating which tools are missing and how to install them. The output of the script is shown to the user before taking them to the lesson selection screen.

In each spoken language directory there are directories for each programming language containing lessons in that spoken language. Each lesson directory contains a number of files. First of all there is the `lesson.yaml` file that contains metadata about the lesson, including the title and the completion status. The `lesson.md` file contains the content of the lesson. The content of the lesson is written in Markdown. Each lesson should include an introduction and a clear description of the task. This may include example code and the success criteria for completing the lesson.

## Writing Lessons

The lesson content may also contain "hints" that are written in the `lesson.md` file like so:

```Markdown
## Hint - This is a small hint for the first step

To complete the first step, use the following code...
```

Hints must begin with an H2 header (e.g. `##`) with the text beginning with "Hint -". The workshop tool will detect this and render the hint collapsed by default like so:

    ▶  ## Hint - This is a small hint for the first step

When the user highlights and hits enter on the hint, it expands to show the content of the hint like so:

    ▼  ## Hint - This is a small hint for the first step
    
    To complete the first step, use the following code...

The hints are not required, but they are recommended to help ensure the people taking your workshop do not get stuck. It is also recommended that at the bottom of every `lesson.md` file you include a hint that shows the complete solution like so:

```markdown
## Hint - Complete Solution

... a description of the valid solution and the code that implements it ...
```

## Checking Solutions

Each lesson must have a way to test the user's solution. We do this using Docker. Inside of each lesson there is a `docker-compose.yaml` file as well as tester folders containing Dockerfiles. The `docker-compose.yaml` file is used to set up the Docker environment for the lesson. It should include a service for each tester folder. Each tester folder contains a `Dockerfile` that builds and runs the tester. To check a user's solution, there is always at least one tester Dockerfile that builds a docker image from the source code in the project directory and runs it. The output from running the user's solution is saved in a `stdout.log` file in the lesson directory. The `check.py` Python script in the lesson directory handles running docker compose to build and run the testers as well as checking the `stdout.log` file for the expected output. The `check.py` script is executed when the user selects the "Check Solution" option in the workshop tool. The script should return a non-zero exit code if the solution does not pass all of the tests. The script should also print a message to the user indicating which tests failed and how to fix them.

You may be wondering why we chose to use Docker instead of another testing framework. The primary reason was so that we can support any programming language and any kind of application programming including networked applications. Using Docker and Docker Compose, we are able to test users' solutions in a Docker network or even a real-world network if required. This is a hard requirement since this tool is designed to teach libp2p programming.

The example workshop that comes in this repository contains a simple workshop consisting of a single lesson that can be completed in Rust, Python, Golang, and JavaScript. The single lesson requires the user to write a solution that prints "Hello, World!" to stdout. There are implementations of the `deps.py` script for each of the programming languages as well as implementations of the `check.py` script for the lessons in each language.

For instance, the lesson in Japanese checks for the string "こんにちは、世界！" in the `stdout.log` file. The lesson in Hindi checks for "नमस्ते, दुनिया!", the lesson in English checks for "Hello, World!", and the lesson in Spanish checks for "¡Hola Mundo!"

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. The hope is that a directory of workshops builds from contributors submitting new workshops. If that's you, please make a post on the Github Discussions for this project.
