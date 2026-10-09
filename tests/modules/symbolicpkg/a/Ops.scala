package spa.`+`

class C(val n: Int)

object Tools:
  def one: C = new C(1)

def twice(c: C): Int = c.n * 2
