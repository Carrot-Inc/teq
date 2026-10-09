
extension (m: Mode)
  def shout: String = m.name.toUpperCase
  def twice: String = m.shout + m.shout

type Elem[X] = X match
  case List[t] => t
  case Option[t] => t

def firsts(xs: List[Int]): List[Int] =
  for
    x <- xs
    if x > 0
  yield x + 1

def nameOf(n: Named { def name: String }): String = n.name

def counted: Int =
  val k = 1
  k
