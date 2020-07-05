#! /usr/bin/env python3

from ctypes import *
lib = cdll.LoadLibrary('../target/debug/librva.so')

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

_drop_buffer = lib.drop_buffer

class Simulation:
    def __init__(self):
        graph_sim = _create_graph_sim()
        self.graph = graph_sim.graph
        self.sim = graph_sim.sim

    def set_value(self, location: str, values: [bool]):
        location = bytes(location, 'utf-8')

        byte_list = c_byte * len(location)
        bool_list = c_bool * len(values)

        path_ptr = byte_list(*location)
        path_len = c_ulonglong(len(location))

        values_ptr = bool_list(*values)
        values_len = c_ulonglong(len(values))


        _set_value(self.sim, self.graph, path_ptr, path_len, values_ptr, values_len)

    def get_value(self, location: str) -> [bool]:
        location = bytes(location, 'utf-8')

        byte_list = c_byte * len(location)

        path_ptr = byte_list(*location)
        path_len = c_ulonglong(len(location))

        buffer_ptr = POINTER(c_bool)()
        length = _get_value(self.sim, self.graph, path_ptr, path_len, byref(buffer_ptr))

        buffer = [False] * length
        for i in range(length):
            buffer[i] = buffer_ptr[i]

        return buffer

    def simulate_unbounded(self):
        _simulate(self.sim, c_ulonglong(0))

    def simulate_bounded(self, bound: int):
        _simulate(self.sim, c_ulonglong(bound))


sim = Simulation()
sim.simulate_unbounded()

print('Testing if the Register is working properly:')
for i in range(16):
    values = [False] * 4;
    for k in range(4):
        values[k] = ((i >> k) & 1) > 0

    sim.set_value('reg.clk', [False])
    sim.simulate_unbounded()
    sim.set_value('reg.d', values)
    sim.simulate_unbounded()
    sim.set_value('reg.clk', [True])
    sim.simulate_unbounded()
    q_values = sim.get_value('reg.q')

    print(str(values) + '\t' + str(q_values))

    assert values == q_values

