object Left:
  class Token
object Right:
  class Token

def symbols(x: Left.Token): Right.Token = x
def constrained[A <: CharSequence](x: A): Int = x

object Alone
object Outer:
  object Inner

def bare: String = Alone
def nested: String = Outer.Inner

object Shadow:
  class Any
  def take(x: scala.Any): Any = x
