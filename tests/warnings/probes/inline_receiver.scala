// An inherited inline given is credited to the import of the object it was read on.
package given_inline
trait Base { inline given n: Int = 1 }
object A extends Base
object B extends Base
object Use {
  import A.n
  def f: Int = {
    import B.given
    summon[Int]
  }
}
