package pfa

// Two files of one package, which --module-per-file makes two modules in both builds.
object Beta:
  val base = { print("beta "); 10 }
  def plus(a: Alpha): Int = a.twice + base
