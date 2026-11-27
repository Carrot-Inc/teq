package a2
class Contra[-A] { def n: Int = 31 }
trait Left extends Contra[Int]
trait Right extends Contra[String]
class Both extends Left with Right
object BO extends Left with Right
