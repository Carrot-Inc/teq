object Probe:
  def main(args: Array[String]): Unit =
    try () finally
      locally {
        def read(n: Int): Int =
          trait Count:
            def count(
              p0: Long, p1: Long, p2: Long, p3: Long, p4: Long, p5: Long, p6: Long, p7: Long,
              p8: Long, p9: Long, p10: Long, p11: Long, p12: Long, p13: Long, p14: Long, p15: Long,
              p16: Long, p17: Long, p18: Long, p19: Long, p20: Long, p21: Long, p22: Long, p23: Long,
              p24: Long, p25: Long, p26: Long, p27: Long, p28: Long, p29: Long, p30: Long, p31: Long,
              p32: Long, p33: Long, p34: Long, p35: Long, p36: Long, p37: Long, p38: Long, p39: Long,
              p40: Long, p41: Long, p42: Long, p43: Long, p44: Long, p45: Long, p46: Long, p47: Long,
              p48: Long, p49: Long, p50: Long, p51: Long, p52: Long, p53: Long, p54: Long, p55: Long,
              p56: Long, p57: Long, p58: Long, p59: Long, p60: Long, p61: Long, p62: Long, p63: Long,
              p64: Long, p65: Long, p66: Long, p67: Long, p68: Long, p69: Long, p70: Long, p71: Long,
              p72: Long, p73: Long, p74: Long, p75: Long, p76: Long, p77: Long, p78: Long, p79: Long,
              p80: Long, p81: Long, p82: Long, p83: Long, p84: Long, p85: Long, p86: Long, p87: Long,
              p88: Long, p89: Long, p90: Long, p91: Long, p92: Long, p93: Long, p94: Long, p95: Long,
              p96: Long, p97: Long, p98: Long, p99: Long, p100: Long, p101: Long, p102: Long, p103: Long,
              p104: Long, p105: Long, p106: Long, p107: Long, p108: Long, p109: Long, p110: Long, p111: Long,
              p112: Long, p113: Long, p114: Long, p115: Long, p116: Long, p117: Long, p118: Long, p119: Long,
              p120: Long, p121: Long, p122: Long, p123: Long, p124: Long, p125: Long, last: Int
            )(x: Int = n): Int = x
          class Reader extends Count
          new Reader().count(0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0L, 0)()
        read(1)
      }

// A method of a local trait takes 126 Longs and an Int before a default that captures another Int.
// The finalizer contains the expression.
// scalac 3.8.4 rejects it: Platform restriction: a parameter list's length cannot exceed 254 (Long and Double count as 2).
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 1
// expect: default_forwarder_with_wide_priors_in_finally.scala:6:17: error: Platform restriction: a parameter list's length cannot exceed 254 (Long and Double count as 2).
// expect: 1 error found
