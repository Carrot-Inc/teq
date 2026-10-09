// Paths through vals declared as singleton types name the same value: `alias: a.type` gives
// `alias.R.type` for `a.R`, and `self: this.type` inside the class gives `self.R.type` for `R`.
class O:
  object R
  val self: this.type = this
  val mine: self.R.type = R

@main def run(): Unit =
  val a = new O
  val alias: a.type = a
  val r: alias.R.type = a.R
  println(r eq a.R)
  println(a.mine eq a.R)
