// A local trait in the body of an inline method reading the method's parameter and a value of the
// block the local inline method is defined in: copied at each expansion with its members' proxies,
// which the copies of the classes mixing it in pass.
inline def made(n: Int): Int =
  trait T:
    def get = n
  (new T {}).get

def viaLocal(m: Int): Int =
  inline def local(n: Int): Int =
    trait T:
      val start = n * 10
      def get = start + n + m
    class C extends T
    (new C).get
  local(1) + local(2)

@main def run(): Unit =
  println(made(3) + made(4))
  println(viaLocal(100))
