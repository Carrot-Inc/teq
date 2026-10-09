package lib

export lib.internal.Strings.{*, given}
export lib.internal.{Version, Pretty}
export internal.Numbers.double as twiceOf

def show[A](a: A)(using p: Pretty[A]): String = p.pretty(a)
