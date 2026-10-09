// A binding of an object's instance (`$$X` for `Xylophone$()`) is named in the scope of the body
// it is declared in, apart from what that body reads that is no local of it: a top-level def of
// the name read after the binding, before it and inside a lambda; and apart from a local of a
// nested block that stands around a read of the object.
object Xylophone:
  def value: Int = 1

def `$$X`(): Int = 7

def before(): Int =
  val first = `$$X`()
  first + Xylophone.value + Xylophone.value + Xylophone.value + Xylophone.value

def nested(flag: Boolean): Int =
  val total = Xylophone.value + Xylophone.value + Xylophone.value + Xylophone.value
  if flag then
    val `$$X` = 5
    total + Xylophone.value + `$$X`
  else total

def inLambda(): List[Int] =
  val base = Xylophone.value + Xylophone.value + Xylophone.value + Xylophone.value
  List(1, 2).map(i => i + base + `$$X`())

@main def main(): Unit =
  println(Xylophone.value + Xylophone.value + Xylophone.value + Xylophone.value)
  println(`$$X`())
  println(before())
  println(nested(true))
  println(nested(false))
  println(inLambda())
