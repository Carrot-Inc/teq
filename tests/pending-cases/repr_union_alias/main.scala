object O:
  type U = 1 | 2
@main def unionAlias(): Unit = println(Reflect.shape[O.U])
