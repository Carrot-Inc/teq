package elsewhere
object D:
  def use(o: nest.Outer)(i: o.Inner): Int =
    i.set(1)
    i.get
