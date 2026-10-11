package enumcompanionbody

// An enum whose companion has a body: reading a case runs it first, from the products too.
enum Level:
  case Low, High

object Level:
  println("Level companion initialised")
  val default: Level = High
