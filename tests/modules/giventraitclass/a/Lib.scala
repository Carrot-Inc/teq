package gtca

// A trait's given class with a using clause, an inner class of the trait made by its given's
// method on the object that mixes the trait in, called over the products.
trait T { def n: Int }
trait Gives:
  given gc(using i: Int): T with
    def n = i
object O extends Gives
