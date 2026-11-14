package rebuilt

def read: String =
  val taken = Counter.take(Inner)
  val a: Inner.type = Counter.alias
  val tag: Counter.held.Tag = new Inner.Tag(4)
  "" + taken + a.twice(Counter.alias.size) + tag.n

@main def run(): Unit = println(read)
