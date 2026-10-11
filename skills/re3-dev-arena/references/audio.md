# Arena ped audio and derived bindings

- Arena ped audio owns its own stage after vehicle audio, before primary animation.
It processes both canonical player peds through CPed access only; vehicle
weather/handling/world dependencies must not gate ped events. Construction
templates cannot retain another actor's physical audio ID or queued footstep.
Derived audio bindings retire outside arena/pause guards, validate current
slot ownership/type, and preserve slots already claimed by another live actor.
