package ilca

// A named local class of an inline body capturing the method's parameter, expanded twice
// downstream: each expansion's class is a class of its own, over scalac's products too.
object API:
  inline def make(n: Int): () => Int =
    class C:
      def get = n
    val c = new C
    () => c.get
