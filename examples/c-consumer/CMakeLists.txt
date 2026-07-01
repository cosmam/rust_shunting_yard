cmake_minimum_required(VERSION 3.16)

project(ShuntingYardFFICConsumer C)

find_package(ShuntingYardFFI REQUIRED)

add_executable(shunting_yard_ffi_c_consumer main.c)

option(SHUNTING_YARD_FFI_LINK_STATIC "Link the static ShuntingYardFFI library" OFF)

if(SHUNTING_YARD_FFI_LINK_STATIC)
  target_link_libraries(
    shunting_yard_ffi_c_consumer
    PRIVATE ShuntingYardFFI::ShuntingYardFFIStatic
  )
else()
  target_link_libraries(
    shunting_yard_ffi_c_consumer
    PRIVATE ShuntingYardFFI::ShuntingYardFFI
  )
endif()
