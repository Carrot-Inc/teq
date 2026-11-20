// An enum's singleton value is a creation: it initializes the Java interface its enum extends.
enum Tone extends jlib.Meter:
  case Low, High

@main def run(): Unit =
  println("before")
  println(Tone.Low)
