object H:
  val ignored: Int = N.value
  trait TC:
    def n: Int
  inline def read(using tc: TC): Int = tc.n
  def value(n: Int): Int =
    val n0 = n
    given TC with
      def n: Int = n0
    trait Local:
      def result: Int = read
    class Child extends Local
    (new Child).result
