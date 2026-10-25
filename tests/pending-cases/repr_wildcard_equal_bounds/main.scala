@main def wildcardEqualBounds(): Unit = println(Reflect.shape[List[? >: Int <: Int]])
