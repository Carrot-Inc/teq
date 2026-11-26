#!/usr/bin/env python3
"""tests/default-getters/desc.py: `javap -s -p` of a library's class files (stdin) -> the default getters'
descriptors, the methods whose names hold `$default$`, each `<class>.<name> <descriptor>`, sorted and joined by ` ; `
(the getters a library writes and the bridges and forwarders it gives them, as the JVM links them), of the classes
something outside can name: a local or anonymous class (a `$` then a digit in its name, scalac's `$1` as teq's own) is
left out (scalac's `$1`, teq's `$<hash>_<n>`), its names being each compiler's own design.
  desc.py < javap.txt          the line's text
  desc.py --self-test          the parser on a planted javap listing"""
import re, sys

# A local or anonymous class's name: scalac's `$1`, teq's `$<hash>_<n>` (`T$n1im7s_162`).
LOCAL = re.compile(r'\$\d|\$[0-9a-z]+_\d+(\$|$)')


def getters(text):
    out, cls, name = set(), None, None
    for line in text.splitlines():
        m = re.match(r'^(?:public |final |abstract |interface )*(?:class|interface) ([\w.$]+)', line.strip())
        if m and line.rstrip().endswith('{'):
            cls = m.group(1)
            continue
        m = re.search(r'\s([\w$]+)\(.*\);$', line)
        if m:
            name = m.group(1)
            continue
        d = re.match(r'\s+descriptor: (.*)$', line)
        if d and name and '$default$' in name and not LOCAL.search(cls or ''):
            out.add(f'{cls}.{name} {d.group(1)}')
            name = None
    return ' ; '.join(sorted(out))


def self_test():
    planted = ('Compiled from "Lib.scala"\npublic interface lib.Base {\n  public default int m(int);\n    descriptor: (I)I\n'
               '  public default int m$default$1();\n    descriptor: ()I\n}\npublic final class lib.C {\n'
               '  public int m$default$1();\n    descriptor: ()I\n  public int other();\n    descriptor: ()I\n}\n'
               'final class lib.Lib$T$1 {\n  public int m$default$1();\n    descriptor: ()I\n}\n'
               'public interface lib.T$n1im7s_162 {\n  public int m$default$1();\n    descriptor: ()I\n}\n')
    got = getters(planted)
    want = 'lib.Base.m$default$1 ()I ; lib.C.m$default$1 ()I'
    if got != want:
        sys.exit(f'desc SELF-TEST FAILED: {got!r}')
    print("desc self-test: a planted listing read (two getters kept; a method, a non-getter and two local classes, scalac's and teq's, left out)")


if __name__ == '__main__':
    if '--self-test' in sys.argv:
        self_test()
    else:
        print(getters(sys.stdin.read()))
