#! /usr/bin/env python3

import sys
from ctypes import *

class GraphSimulation(Structure):
    _fields_ = [("graph", c_void_p),("sim", c_void_p)]


def bools(value: int, size: int):
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
    def _load_lib(self):
        lib = cdll.LoadLibrary('../target/release/librva.so')
        self._lib = lib

        # Function Definitions
        lib.create_graph_simulation.restype = GraphSimulation
        
        lib.simulate.argtypes = c_void_p,c_ulonglong
        lib.simulate.restype = c_bool

        lib.get_value.argtypes = c_void_p,c_void_p,c_void_p,c_ulonglong,POINTER(POINTER(c_bool))
        lib.get_value.restype = c_size_t
        
        lib.set_value = lib.set_value
        lib.set_value.argtypes = c_void_p,c_void_p,c_void_p,c_ulonglong,c_void_p,c_ulonglong
        
        lib.get_description.argtypes = c_void_p,c_void_p,c_void_p,c_ulonglong,c_void_p
        lib.get_description.restype = c_size_t
        
        lib.get_width.argtypes = c_void_p,c_void_p,c_ulonglong
        lib.get_width.restype = c_ulonglong
        
        lib.drop_bools.argtypes = c_void_p,c_size_t
        lib.drop_chars.argtypes = c_void_p,c_size_t

    def __init__(self):
        self._load_lib()

        graph_sim = self._lib.create_graph_simulation()
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

        self._lib.set_value(self._sim, self._graph, path_ptr, path_len, values_ptr, values_len)

        if self.autorun_enabled:
            self.run(self.autorun_bound)

    def get_value(self, location: str) -> [bool]:
        path_ptr,path_len = str_to_ptr(location)

        buffer_ptr = POINTER(c_bool)()
        length = self._lib.get_value(self._sim, self._graph, path_ptr, path_len, byref(buffer_ptr))

        buffer = [False] * length
        for i in range(length):
            buffer[i] = buffer_ptr[i]

        self._lib.drop_bools(buffer_ptr, length)

        return buffer

    def run(self, bound:int = 0):
        self._lib.simulate(self._sim, c_ulonglong(bound))

    def get_description(self, location: str):
        path_ptr,path_len = str_to_ptr(location)
        desc_ptr = POINTER(c_byte)()
        length = self._lib.get_description(self._sim, self._graph, path_ptr, path_len, byref(desc_ptr))

        description = ''
        for i in range(length):
            description += chr(desc_ptr[i])

        self._lib.drop_chars(desc_ptr, length)
        return description

    def get_width(self, location):
        path_ptr,path_len = str_to_ptr(location)
        return self._lib.get_width(self._graph, path_ptr, path_len)

class Node(object):
    def __init__(self, sim, path = None):
        super(Node, self).__setattr__('_path', path)
        super(Node, self).__setattr__('_sim', sim)

    def __getattr__(self, name):
        path = super(Node, self).__getattribute__('_path')
        sim  = super(Node, self).__getattribute__('_sim')
        if path is None:
            return Node(sim, name)
        else:
            return Node(sim, path + '.' + name)

    def __repr__(self):
        path = super(Node, self).__getattribute__('_path')
        sim = super(Node, self).__getattribute__('_sim')
        if path is None:
            path = ''
        return sim.get_description(path)

    def __setattr__(self, name, value):
        path = super(Node, self).__getattribute__('_path')
        sim  = super(Node, self).__getattribute__('_sim')
        sim.set_value(path + '.' + name, value)


sys.ps1 = 'rva> '
simulation = Simulation()
top = Node(simulation)

