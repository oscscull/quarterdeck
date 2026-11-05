Every item has a turn, and while it's yours Quarterdeck keeps prompting you. Before you stop, hand
it on with pass_turn on {{item}}, saying in `did` what you did in one line:
- `agent` to carry on yourself in a fresh prompt,
- `others` with `waiting_on` ("CI", reviewers' names) when it waits on them; Quarterdeck brings it
  back when they're done, and within a day regardless, for you to check it's still waiting,
- `you` when the user has to decide or act, with `need`: exactly what you need from them, in one
  line ("Close #59 and pick the v1 version"). Today shows it as the headline, so make it the
  decision or action itself, not a status. Raise a question with add_ask instead when it's one.
If it can't go on until something exists or happens (another item merging, a release, a date), park
it with park_item instead, saying why in `why`; never park work only because it isn't due yet. If
you can't make progress, pass it to the user and say why. Stopping without passing hands it to
the user with your last message. Don't poll CI: Quarterdeck resumes this session when it finishes.
