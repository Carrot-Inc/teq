object Part11:
  def pad11(n: Int): Int = n + 11
  def run(n: Int): Int =
    def first: Int = second + pad11(n)
    val held: Int = first
    def second: Int = n
    held
