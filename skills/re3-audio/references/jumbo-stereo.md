# Jumbo audio and real stereo voices

- `JumboAudioState` owns native gJumboVolOffsetPercentage once per app.
  Plane audio reads plane_id/speed and CPlaneRuntime's authored flight path
  positions/markers; do not select phases from invented speed thresholds or
  copy live path state into an independent cache. Original SDT loop metadata
  remains authoritative. Native jumbo 2D endpoints route installed PCM into
  actual left/right AudioSource channels. Cache keys include sample, full/sustain
  region and side; attack and sustain use the same routing. Gain/pitch updates
  retain live voices; pause and retirement remove every loop/attack voice.
  Do not play both endpoint requests as centered mono. Invalid endpoint inputs
  remain observable; request queues alone do not certify real stereo sinks.
