// A required type scalac wraps over two lines: its payload is the whole type.
trait VeryLongTypeNameToForceThePrettyPrinterIntoMultilineModeOne
trait VeryLongTypeNameToForceThePrettyPrinterIntoMultilineModeTwo
def f(x: Int): Either[VeryLongTypeNameToForceThePrettyPrinterIntoMultilineModeOne, VeryLongTypeNameToForceThePrettyPrinterIntoMultilineModeTwo] = x
