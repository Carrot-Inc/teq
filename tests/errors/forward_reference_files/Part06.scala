object Part06:
  def pad6(n: Int): Int = n + 6
  def run(n: Int): Int =
    def first: Int = second + pad6(n)
    val held: Int = first
    def second: Int = n
    held
