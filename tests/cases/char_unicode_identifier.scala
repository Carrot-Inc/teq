// `Character.isUnicodeIdentifierStart` and `isUnicodeIdentifierPart`: letters and letter numbers
// start an identifier; digits, connectors, marks and the ignorable controls continue one.
@main def run =
  for c <- List('a', 'Z', 'é', 'Ⅻ', '_', '$', '1', '-', ' ', '́', '\u0000', '​') do
    println(s"${c.toInt} ${Character.isUnicodeIdentifierStart(c)} ${Character.isUnicodeIdentifierPart(c)}")
