package kbx

object UserSide:
  given Mode = Mode(700)
  def run(): Unit = println(Lib.value)

@main def run(): Unit = UserSide.run()
