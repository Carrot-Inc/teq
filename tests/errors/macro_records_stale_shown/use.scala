package shown

def first: () => String = () => shown
def second: () => String = () => shown
// expect: a symbol of another expansion's site was used again
// expect: use.scala:4:34
// expect: (called by shown)
