package recmu

import recm.Api

@main def run(): Unit =
  println(Inspect.recursive[Api.S])
  println(Inspect.recursive[Api.R])
  println(Inspect.plain[Api.P])
