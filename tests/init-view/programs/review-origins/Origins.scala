trait Show[A]
class Holder[A: Show](using s: Show[A])
