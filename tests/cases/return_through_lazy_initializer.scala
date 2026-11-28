class Sample:
  lazy val value: Int =
    try
      println("initialize")
      40
    finally println("initializer cleanup")

  def read(): Int =
    try return value + 2
    finally println("read cleanup")

@main def run(): Unit =
  val sample = new Sample
  println(sample.read())
  println(sample.read())
