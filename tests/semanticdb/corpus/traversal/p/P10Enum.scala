package p10
enum Color(val rgb: Int):
  case Red extends Color(1)
  case Green extends Color(2)
  case Custom(v: Int) extends Color(v)
enum Simple:
  case A, B
  case C(x: Int)
  def m = 1
enum Opt[+T]:
  case Sm(x: T)
  case Nn
