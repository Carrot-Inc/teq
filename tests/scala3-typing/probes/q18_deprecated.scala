object T:
  @deprecated("use fresh", "1.0") def old = 1
  def fresh = 2
@main def run(): Unit = println(T.old)
