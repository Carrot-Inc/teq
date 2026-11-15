package inc
def read: String = summon[Show[Marker]].show + Stock.seed
