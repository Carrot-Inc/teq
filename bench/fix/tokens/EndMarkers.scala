package endmarkers

object EndMarkerKeywords:
  def loop(n: Int): Unit =
    var i = n
    while i > 0 do
      i -= 1
    end while
  def cond(b: Boolean): Int =
    if b then
      1
    else
      2
    end if
  def pick(x: Int): Int =
    x match
      case 0 => 1
      case _ => 2
    end match
  def each(): Unit =
    for k <- List(1) do
      println(k)
    end for
  def guard(): Int =
    try
      1
    finally
      println("done")
    end try
  def anon(): Object =
    new Object:
      override def toString = "o"
    end new
  class K(i: Int):
    def this() =
      this(1)
    end this
  val v =
    1
  end v
  given Ordering[K] with
    def compare(a: K, b: K) = 0
  end given
end EndMarkerKeywords
