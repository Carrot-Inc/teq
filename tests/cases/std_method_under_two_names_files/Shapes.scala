package shapes
class Both extends Joined { def run(): Unit = println("class") }
class Middle extends Both
class Leaf extends Middle
object Single extends Joined { def run(): Unit = println("object") }
enum Kind extends Joined { case One; def run(): Unit = println("enum") }
case class Record(n: Int) extends Joined { def run(): Unit = println("case") }
