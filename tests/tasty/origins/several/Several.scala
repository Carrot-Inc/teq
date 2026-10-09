package sv

class Several(val n: Int):
  def twice: Int = n * 2

object Several:
  def of(n: Int): Several = new Several(n)

def top(s: Several): Int = s.twice
