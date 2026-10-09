package roots.use

/** The program's one call of `productElement` and its one `getClass`. */
object Use:
  def first(p: Product): Any = p.productElement(0)
  def named(x: Any): String = x.getClass.getName
  def label: String = "use"
