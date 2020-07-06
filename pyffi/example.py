#! /usr/bin/env python3

import sys
sys.ps1 = 'rva> '

from ctypes import *
lib = cdll.LoadLibrary('../target/release/librva.so')

# Type Definitions
class GraphSimulation(Structure):
    _fields_ = [("graph", c_void_p),("sim", c_void_p)]

# Function Definitions
_create_graph_sim = lib.create_graph_simulation
_create_graph_sim.restype = GraphSimulation

_simulate = lib.simulate
_simulate.argtypes = c_void_p,c_ulonglong
_simulate.restype = c_bool

_get_value = lib.get_value
_get_value.argtypes = c_void_p,c_void_p,c_void_p,c_ulonglong,POINTER(POINTER(c_bool))
_get_value.restype = c_size_t

_set_value = lib.set_value
_set_value.argtypes = c_void_p,c_void_p,c_void_p,c_ulonglong,c_void_p,c_ulonglong

_get_description = lib.get_description
_get_description.argtypes = c_void_p,c_void_p,c_void_p,c_ulonglong
_get_description.restype = c_char_p

_get_width = lib.get_width
_get_width.argtypes = c_void_p,c_void_p,c_ulonglong
_get_width.restype = c_ulonglong

_drop_buffer = lib.drop_buffer

def bools(value: int, size: int):
    if value <= 0:
        return [False]
    else:
        bools = []
        for i in range(size):
            bools.append(value & 1 > 0)
            value = value >> 1
        return bools

def str_to_ptr(string: str):
    string = bytes(string, 'utf-8')
    byte_list = c_byte * len(string)
    return (byte_list(*string), len(string))

class Simulation:
    def __init__(self):
        graph_sim = _create_graph_sim()
        self._graph = graph_sim.graph
        self._sim = graph_sim.sim

        self.autorun_enabled = True
        self.autorun_bound = 1000


    def set_value(self, location: str, value: int):
        width = self.get_width(location)
        values = bools(value, width)
        bool_list = c_bool * len(values)

        path_ptr,path_len = str_to_ptr(location)

        values_ptr = bool_list(*values)
        values_len = c_ulonglong(len(values))

        _set_value(self._sim, self._graph, path_ptr, path_len, values_ptr, values_len)

        if self.autorun_enabled:
            self.run(self.autorun_bound)

    def get_value(self, location: str) -> [bool]:
        path_ptr,path_len = str_to_ptr(location)

        buffer_ptr = POINTER(c_bool)()
        length = _get_value(self._sim, self._graph, path_ptr, path_len, byref(buffer_ptr))

        buffer = [False] * length
        for i in range(length):
            buffer[i] = buffer_ptr[i]

        return buffer

    def run(self, bound:int = 0):
        _simulate(self._sim, c_ulonglong(bound))

    def get_description(self, location: str):
        path_ptr,path_len = str_to_ptr(location)
        description = _get_description(self._sim, self._graph, path_ptr, path_len)
        return description.decode('utf-8') 

    def get_width(self, location):
        path_ptr,path_len = str_to_ptr(location)
        return _get_width(self._graph, path_ptr, path_len)

class Node(object):
    def __init__(self, sim, path = None):
        super(Node, self).__setattr__('__sim', sim)
        super(Node, self).__setattr__('__path', path)

    def __getattr__(self, name):
        path = super(Node, self).__getattribute__('__path')
        sim  = super(Node, self).__getattribute__('__sim')
        if path is None:
            return Node(sim, name)
        else:
            return Node(sim, path + '.' + name)

    def __repr__(self):
        path = super(Node, self).__getattribute__('__path')
        sim = super(Node, self).__getattribute__('__sim')
        if path is None:
            path = ''
        return sim.get_description(path)

    def __setattr__(self, name, value):
        path = super(Node, self).__getattribute__('__path')
        sim  = super(Node, self).__getattribute__('__sim')
        sim.set_value(path + '.' + name, value)

simulation = Simulation()
top = Node(simulation)




