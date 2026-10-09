package app
import envlib.Env

@main def run(): Unit =
  println(Env.probe("TEQ_CASE_VARIABLE_NEVER_SET"))
  println(Env.probe("PATH"))
