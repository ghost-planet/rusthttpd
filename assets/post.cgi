#!/usr/bin/python

#coding:utf-8
import sys,os
length = os.getenv('CONTENT_LENGTH')
postdata = sys.stdin.read(int(length))

print "Content-type:text/html\n"
print '<html>'
print '<head>'
print '<title>POST</title>'
print '</head>'
print '<body>'
print '<ul>'
for data in postdata.split('&'):
    print  '<li>'+data+'</li>'
print '</ul>'
print '</body>'
print '</html>'