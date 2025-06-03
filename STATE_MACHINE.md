# Workshop State Machine

The workshop app follows a strict state machine illustrated below.

```

            ┌─────┐
            │ Nil │
            └──┬──┘
               │
            <Config>
               │
               │
               │
               │                             ┌──────────────────────────────────────────────────────┐
 ╟──<quit>──┐  │  ┌───────────<back>─────────┤ SetProgrammingLanguageDefault (programming_language) │
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
 │                             ├─────<GetLicense>────>│                            │
 │ SelectWorkshop (workshops)  │                      │ ShowLicense (license_text) │
 │                             │<───────<Back>────────┤                            │
 │                             │                      └────────────────────────────┘
 │                             │
 │                             ├────<ChangeSpokenLanguage>─────────┐
 └─────────────┬───────────────┘                                   ▼
            ▲  │  ▲                            ┌─────────────────────────────────────────┐
            │  │  │                            │ SelectSpokenLanguage (spoken_languages) │
         <back>│  │                            └───────────────────┬─────────────────────┘
            │  │  │                                                │
            │  │  │                                       <SetSpokenLanguage>
            │  │  │                                                │
            │  │  │                                                ▼
            │  │  │                           ┌────────────────────────────────────────────┐
            │  │  └───────────<back>──────────┤ SetSpokenLanguageDefault (spoken_language) │
            │  │                              └────────────────────────────────────────────┘
         <SetWorkshop>
            │  │
            │  ▼
   ┌────────┴───────────────┐
   │ SelectLesson (lessons) │
   └───────────┬────────────┘
            ▲  │  ▲                                          ┌────────────────┐
            │  │  └──────────────────<back>──────────────────┤ LessonComplete │
         <back>│                                             └────────────────┘
            │  │                                                     ▲
            │  │                                                     │
            │  │                                                 [Complete]
            │  ▼                                                     │
  ┌─────────┴────────────────┐                       ┌───────────────┴────────────────┐
  │ ShowLesson (lesson_text) ├─────<CheckLesson>────>│ CheckLesson (task, log_handle) │
  └──────────────────────────┘                       └───────────────┬────────────────┘
               ▲                                                     │
               │                                                 [Failure]
               │                                                     │
               │                                                     ▼
               │                                            ┌──────────────────┐
               └─────────────────────<back>─────────────────┤ LessonIncomplete │
                                                            └──────────────────┘
```
