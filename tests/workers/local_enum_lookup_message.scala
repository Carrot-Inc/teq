// jars: scala-library
// A local enum's `fromOrdinal` names the enum in its message by the place of the enum's class (dotty's
// `enumClass.fullName`, under the file's object): the name the typing keeps by the class, a class of a block
// that a worker numbers and the merge numbers again.
def pick(n: Int): String =
  enum Tone:
    case Low(x: Int)
  try Tone.fromOrdinal(n).toString
  catch case e: NoSuchElementException => e.getMessage

@main def run(): Unit = println(pick(0))
