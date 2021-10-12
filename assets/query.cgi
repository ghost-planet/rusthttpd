#!/usr/bin/python

#coding:utf-8
import sys,os
query_string = os.getenv('QUERY_STRING')
if query_string:
    print "Content-type:text/html\n"

    print "<!DOCTYPE html>"
    print '<html lang="en">'
    print "<head>"
    print '<meta charset="utf-8">'
    print '<title>Internal Server Error</title>'
    print '</head>'
    print '<body>'
    print '<p>Query string:' + query_string + '.</p>'
    print '</body>'
    print '</html>'

else:
    print "Content-type:text/html\n"
    print 'no found'