# Workshop State Machine

The workshop app follows a strict state machine illustrated below.

```

            ┌─────┐
            │ Nil │
            └──┬──┘
               │
        <LoadWorkshops>
               │
               │
               │
               │                             ┌──────────────────────────────────────────────────────┐
╟──<quit>───┐  │  ┌─────<LoadWorkshops>──────┤ SetProgrammingLanguageDefault (programming_language) │
            │  │  │                          └──────────────────────────────────────────────────────┘
            │  │  │                                                Ʌ
            │  │  │                                                │
            │  │  │                                     <SetProgrammingLanguage>
            │  │  │                                                │
            │  │  │                            ┌───────────────────┴───────────────────────────────┐
            │  │  │                            │ SelectProgrammingLanguage (programming_languages) │
            │  V  V                            └───────────────────────────────────────────────────┘
 ┌──────────┴──────────────────┐                                   Ʌ
 │                             ├────<ChangeProgrammingLanguage>────┘
 │                             │
 │                             │                      ┌────────────────────────────┐
 │                             ├────<ShowLicense>────>│                            │
 │ SelectWorkshop (workshops)  │                      │ ShowLicense (license_text) │
 │                             │<───<LoadWorkshops>───┤                            │
 │                             │                      └────────────────────────────┘
 │                             │
 │                             ├────<ChangeSpokenLanguage>─────────┐
 └─────────────┬───────────────┘                                   V
            Ʌ  │  Ʌ                            ┌─────────────────────────────────────────┐
            │  │  │                            │ SelectSpokenLanguage (spoken_languages) │
<LoadWorkshops>│  │                            └───────────────────┬─────────────────────┘
            │  │  │                                                │
            │  │  │                                       <SetSpokenLanguage>
            │  │  │                                                │
            │  │  │                                                V
            │  │  │                           ┌────────────────────────────────────────────┐
            │  │  └───────<LoadWorkshops>─────┤ SetSpokenLanguageDefault (spoken_language) │
            │  │                              └────────────────────────────────────────────┘
            │<LoadLessons>
╟─<quit>─┐  │  │
         │  │  V
   ┌─────┴──┴───────────────┐
   │ SelectLesson (lessons) │
   └───────────┬────────────┘
            Ʌ  │  Ʌ                                          ┌────────────────┐
            │  │  └────────────────<LoadLessons>─────────────┤ LessonComplete │
  <LoadLessons>│                                             └────────────────┘
            │  │                                                     Ʌ
            │<LoadLesson>                                            │
╟─<quit>─┐  │  │                                                 [Complete]
         │  │  V                                                     │
  ┌──────┴──┴────────────────┐                       ┌───────────────┴────────────────┐
  │ ShowLesson (lesson_text) ├─────<CheckLesson>────>│ CheckLesson (task, log_handle) │
  └──────────────────────────┘                       └───────────────┬────────────────┘
               Ʌ                                                     │
               │                                                 [Failure]
               │                                                     │
               │                                                     V
               │                                            ┌──────────────────┐
               └───────────────────<LoadLesson>─────────────┤ LessonIncomplete │
                                                            └──────────────────┘
```
