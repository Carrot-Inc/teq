class C:
  def apply: C = this
object Test:
  def t = (new C)(22)
@main def run(): Unit = println(1)
