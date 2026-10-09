package xa

class Engine:
  def start(power: Int): String = "vroom " + power
  def name: String = "engine"

object DefaultEngine extends Engine

class Car:
  export DefaultEngine.{start, name as engineName}

object Tools:
  val size: Int = 3
  def hammer: String = "hammer"
  class Nail(val length: Int)
object Kit:
  export Tools.*
