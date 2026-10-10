package names

// Members a user named like inline accessors keep their own simple names over the products,
// beside the accessor dotty's `PrepareInlineable` makes for `count`, whose symbol alone has
// the derived name (`accessorNameOf`: `INLINEACCESSOR(EXPANDED(names$Api, count))`).
class Api:
  private var count = 0
  def inline$owner$$member: Int = 7
  def inline$plain: Int = 8
  inline def bump(): Int =
    count += 1
    count
