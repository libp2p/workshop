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
            │  │  │                                                ▲
            │  │  │                                                │
            │  │  │                                     <SetProgrammingLanguage>
            │  │  │                                                │
            │  │  │                            ┌───────────────────┴───────────────────────────────┐
            │  │  │                            │ SelectProgrammingLanguage (programming_languages) │
            │  ▼  ▼                            └───────────────────────────────────────────────────┘
 ┌──────────┴──────────────────┐                                   ▲
 │                             ├────<ChangeProgrammingLanguage>────┘
 │                             │
 │                             │                      ┌────────────────────────────┐
 │                             ├────<ShowLicense>────>│                            │
 │ SelectWorkshop (workshops)  │                      │ ShowLicense (license_text) │
 │                             │<───<LoadWorkshops>───┤                            │
 │                             │                      └────────────────────────────┘
 │                             │
 │                             ├────<ChangeSpokenLanguage>─────────┐
 └─────────────┬───────────────┘                                   ▼
            ▲  │  ▲                            ┌─────────────────────────────────────────┐
            │  │  │                            │ SelectSpokenLanguage (spoken_languages) │
<LoadWorkshops>│  │                            └───────────────────┬─────────────────────┘
            │  │  │                                                │
            │  │  │                                       <SetSpokenLanguage>
            │  │  │                                                │
            │  │  │                                                ▼
            │  │  │                           ┌────────────────────────────────────────────┐
            │  │  └───────<LoadWorkshops>─────┤ SetSpokenLanguageDefault (spoken_language) │
            │  │                              └────────────────────────────────────────────┘
            │<SetWorkshop>
╟─<quit>─┐  │  │
         │  │  ▼
   ┌─────┴──┴───────────────┐
   │ SelectLesson (lessons) │
   └───────────┬────────────┘
            ▲  │  ▲                                          ┌────────────────┐
            │  │  └────────────────<LoadLessons>─────────────┤ LessonComplete │
  <LoadLessons>│                                             └────────────────┘
            │  │                                                     ▲
            │<SetLesson>                                             │
╟─<quit>─┐  │  │                                                 [Complete]
         │  │  ▼                                                     │
  ┌──────┴──┴────────────────┐                       ┌───────────────┴────────────────┐
  │ ShowLesson (lesson_text) ├─────<CheckLesson>────>│ CheckLesson (task, log_handle) │
  └──────────────────────────┘                       └───────────────┬────────────────┘
               ▲                                                     │
               │                                                 [Failure]
               │                                                     │
               │                                                     ▼
               │                                            ┌──────────────────┐
               └───────────────────<SetLesson>──────────────┤ LessonIncomplete │
                                                            └──────────────────┘
```
