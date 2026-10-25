object O:
  type P = Int
@main def aliasReference(): Unit = println(Reflect.shape[O.P])
