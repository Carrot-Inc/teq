package outerpk.innerpkg

def twice(x: Int): Int = x.times(2)

@main def main(): Unit = println((twice(21), unit))
