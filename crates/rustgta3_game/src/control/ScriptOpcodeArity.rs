//! Auto-generated from re3 ScriptDebug REGISTER_COMMAND input/output arities.
//! Input/output metadata for SCM decoding; execution rejects unported commands.
//! Do not edit by hand — regenerate from ScriptDebug.cpp when opcodes change.

/// Each entry: (opcode_id, input_kinds, output_count)
/// input_kinds: 0 = CollectParameters-style typed arg, 1 = raw 8-byte script key/string
pub const SCM_OPCODE_ARITY: &[(i32, &[u8], u8)] = &[
    (0, &[], 0),                                                    // NOP
    (1, &[0], 0),                                                   // WAIT
    (2, &[0], 0),                                                   // GOTO
    (3, &[0], 0),                                                   // SHAKE_CAM
    (4, &[0, 0], 0),                                                // SET_VAR_INT
    (5, &[0, 0], 0),                                                // SET_VAR_FLOAT
    (6, &[0, 0], 0),                                                // SET_LVAR_INT
    (7, &[0, 0], 0),                                                // SET_LVAR_FLOAT
    (8, &[0, 0], 0),                                                // ADD_VAL_TO_INT_VAR
    (9, &[0, 0], 0),                                                // ADD_VAL_TO_FLOAT_VAR
    (10, &[0, 0], 0),                                               // ADD_VAL_TO_INT_LVAR
    (11, &[0, 0], 0),                                               // ADD_VAL_TO_FLOAT_LVAR
    (12, &[0, 0], 0),                                               // SUB_VAL_FROM_INT_VAR
    (13, &[0, 0], 0),                                               // SUB_VAL_FROM_FLOAT_VAR
    (14, &[0, 0], 0),                                               // SUB_VAL_FROM_INT_LVAR
    (15, &[0, 0], 0),                                               // SUB_VAL_FROM_FLOAT_LVAR
    (16, &[0, 0], 0),                                               // MULT_INT_VAR_BY_VAL
    (17, &[0, 0], 0),                                               // MULT_FLOAT_VAR_BY_VAL
    (18, &[0, 0], 0),                                               // MULT_INT_LVAR_BY_VAL
    (19, &[0, 0], 0),                                               // MULT_FLOAT_LVAR_BY_VAL
    (20, &[0, 0], 0),                                               // DIV_INT_VAR_BY_VAL
    (21, &[0, 0], 0),                                               // DIV_FLOAT_VAR_BY_VAL
    (22, &[0, 0], 0),                                               // DIV_INT_LVAR_BY_VAL
    (23, &[0, 0], 0),                                               // DIV_FLOAT_LVAR_BY_VAL
    (24, &[0, 0], 0),                    // IS_INT_VAR_GREATER_THAN_NUMBER
    (25, &[0, 0], 0),                    // IS_INT_LVAR_GREATER_THAN_NUMBER
    (26, &[0, 0], 0),                    // IS_NUMBER_GREATER_THAN_INT_VAR
    (27, &[0, 0], 0),                    // IS_NUMBER_GREATER_THAN_INT_LVAR
    (28, &[0, 0], 0),                    // IS_INT_VAR_GREATER_THAN_INT_VAR
    (29, &[0, 0], 0),                    // IS_INT_LVAR_GREATER_THAN_INT_LVAR
    (30, &[0, 0], 0),                    // IS_INT_VAR_GREATER_THAN_INT_LVAR
    (31, &[0, 0], 0),                    // IS_INT_LVAR_GREATER_THAN_INT_VAR
    (32, &[0, 0], 0),                    // IS_FLOAT_VAR_GREATER_THAN_NUMBER
    (33, &[0, 0], 0),                    // IS_FLOAT_LVAR_GREATER_THAN_NUMBER
    (34, &[0, 0], 0),                    // IS_NUMBER_GREATER_THAN_FLOAT_VAR
    (35, &[0, 0], 0),                    // IS_NUMBER_GREATER_THAN_FLOAT_LVAR
    (36, &[0, 0], 0),                    // IS_FLOAT_VAR_GREATER_THAN_FLOAT_VAR
    (37, &[0, 0], 0),                    // IS_FLOAT_LVAR_GREATER_THAN_FLOAT_LVAR
    (38, &[0, 0], 0),                    // IS_FLOAT_VAR_GREATER_THAN_FLOAT_LVAR
    (39, &[0, 0], 0),                    // IS_FLOAT_LVAR_GREATER_THAN_FLOAT_VAR
    (40, &[0, 0], 0),                    // IS_INT_VAR_GREATER_OR_EQUAL_TO_NUMBER
    (41, &[0, 0], 0),                    // IS_INT_LVAR_GREATER_OR_EQUAL_TO_NUMBER
    (42, &[0, 0], 0),                    // IS_NUMBER_GREATER_OR_EQUAL_TO_INT_VAR
    (43, &[0, 0], 0),                    // IS_NUMBER_GREATER_OR_EQUAL_TO_INT_LVAR
    (44, &[0, 0], 0),                    // IS_INT_VAR_GREATER_OR_EQUAL_TO_INT_VAR
    (45, &[0, 0], 0),                    // IS_INT_LVAR_GREATER_OR_EQUAL_TO_INT_LVAR
    (46, &[0, 0], 0),                    // IS_INT_VAR_GREATER_OR_EQUAL_TO_INT_LVAR
    (47, &[0, 0], 0),                    // IS_INT_LVAR_GREATER_OR_EQUAL_TO_INT_VAR
    (48, &[0, 0], 0),                    // IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_NUMBER
    (49, &[0, 0], 0),                    // IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_NUMBER
    (50, &[0, 0], 0),                    // IS_NUMBER_GREATER_OR_EQUAL_TO_FLOAT_VAR
    (51, &[0, 0], 0),                    // IS_NUMBER_GREATER_OR_EQUAL_TO_FLOAT_LVAR
    (52, &[0, 0], 0),                    // IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_FLOAT_VAR
    (53, &[0, 0], 0),                    // IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_FLOAT_LVAR
    (54, &[0, 0], 0),                    // IS_FLOAT_VAR_GREATER_OR_EQUAL_TO_FLOAT_LVAR
    (55, &[0, 0], 0),                    // IS_FLOAT_LVAR_GREATER_OR_EQUAL_TO_FLOAT_VAR
    (56, &[0, 0], 0),                    // IS_INT_VAR_EQUAL_TO_NUMBER
    (57, &[0, 0], 0),                    // IS_INT_LVAR_EQUAL_TO_NUMBER
    (58, &[0, 0], 0),                    // IS_INT_VAR_EQUAL_TO_INT_VAR
    (59, &[0, 0], 0),                    // IS_INT_LVAR_EQUAL_TO_INT_LVAR
    (60, &[0, 0], 0),                    // IS_INT_VAR_EQUAL_TO_INT_LVAR
    (61, &[0, 0], 0),                    // IS_INT_VAR_NOT_EQUAL_TO_NUMBER
    (62, &[0, 0], 0),                    // IS_INT_LVAR_NOT_EQUAL_TO_NUMBER
    (63, &[0, 0], 0),                    // IS_INT_VAR_NOT_EQUAL_TO_INT_VAR
    (64, &[0, 0], 0),                    // IS_INT_LVAR_NOT_EQUAL_TO_INT_LVAR
    (65, &[0, 0], 0),                    // IS_INT_VAR_NOT_EQUAL_TO_INT_LVAR
    (66, &[0, 0], 0),                    // IS_FLOAT_VAR_EQUAL_TO_NUMBER
    (67, &[0, 0], 0),                    // IS_FLOAT_LVAR_EQUAL_TO_NUMBER
    (68, &[0, 0], 0),                    // IS_FLOAT_VAR_EQUAL_TO_FLOAT_VAR
    (69, &[0, 0], 0),                    // IS_FLOAT_LVAR_EQUAL_TO_FLOAT_LVAR
    (70, &[0, 0], 0),                    // IS_FLOAT_VAR_EQUAL_TO_FLOAT_LVAR
    (71, &[0, 0], 0),                    // IS_FLOAT_VAR_NOT_EQUAL_TO_NUMBER
    (72, &[0, 0], 0),                    // IS_FLOAT_LVAR_NOT_EQUAL_TO_NUMBER
    (73, &[0, 0], 0),                    // IS_FLOAT_VAR_NOT_EQUAL_TO_FLOAT_VAR
    (74, &[0, 0], 0),                    // IS_FLOAT_LVAR_NOT_EQUAL_TO_FLOAT_LVAR
    (75, &[0, 0], 0),                    // IS_FLOAT_VAR_NOT_EQUAL_TO_FLOAT_LVAR
    (76, &[0], 0),                       // GOTO_IF_TRUE
    (77, &[0], 0),                       // GOTO_IF_FALSE
    (78, &[], 0),                        // TERMINATE_THIS_SCRIPT
    (79, &[0], 0),                       // START_NEW_SCRIPT
    (80, &[0], 0),                       // GOSUB
    (81, &[], 0),                        // RETURN
    (82, &[0, 0, 0, 0, 0, 0], 0),        // LINE
    (83, &[0, 0, 0, 0], 1),              // CREATE_PLAYER (StoreParameters 1 — player index)
    (84, &[0], 3),                       // GET_PLAYER_COORDINATES
    (85, &[0, 0, 0, 0], 0),              // SET_PLAYER_COORDINATES
    (86, &[0, 0, 0, 0, 0, 0], 0),        // IS_PLAYER_IN_AREA_2D
    (87, &[0, 0, 0, 0, 0, 0, 0, 0], 0),  // IS_PLAYER_IN_AREA_3D
    (88, &[0, 0], 0),                    // ADD_INT_VAR_TO_INT_VAR
    (89, &[0, 0], 0),                    // ADD_FLOAT_VAR_TO_FLOAT_VAR
    (90, &[0, 0], 0),                    // ADD_INT_LVAR_TO_INT_LVAR
    (91, &[0, 0], 0),                    // ADD_FLOAT_LVAR_TO_FLOAT_LVAR
    (92, &[0, 0], 0),                    // ADD_INT_VAR_TO_INT_LVAR
    (93, &[0, 0], 0),                    // ADD_FLOAT_VAR_TO_FLOAT_LVAR
    (94, &[0, 0], 0),                    // ADD_INT_LVAR_TO_INT_VAR
    (95, &[0, 0], 0),                    // ADD_FLOAT_LVAR_TO_FLOAT_VAR
    (96, &[0, 0], 0),                    // SUB_INT_VAR_FROM_INT_VAR
    (97, &[0, 0], 0),                    // SUB_FLOAT_VAR_FROM_FLOAT_VAR
    (98, &[0, 0], 0),                    // SUB_INT_LVAR_FROM_INT_LVAR
    (99, &[0, 0], 0),                    // SUB_FLOAT_LVAR_FROM_FLOAT_LVAR
    (100, &[0, 0], 0),                   // SUB_INT_VAR_FROM_INT_LVAR
    (101, &[0, 0], 0),                   // SUB_FLOAT_VAR_FROM_FLOAT_LVAR
    (102, &[0, 0], 0),                   // SUB_INT_LVAR_FROM_INT_VAR
    (103, &[0, 0], 0),                   // SUB_FLOAT_LVAR_FROM_FLOAT_VAR
    (104, &[0, 0], 0),                   // MULT_INT_VAR_BY_INT_VAR
    (105, &[0, 0], 0),                   // MULT_FLOAT_VAR_BY_FLOAT_VAR
    (106, &[0, 0], 0),                   // MULT_INT_LVAR_BY_INT_LVAR
    (107, &[0, 0], 0),                   // MULT_FLOAT_LVAR_BY_FLOAT_LVAR
    (108, &[0, 0], 0),                   // MULT_INT_VAR_BY_INT_LVAR
    (109, &[0, 0], 0),                   // MULT_FLOAT_VAR_BY_FLOAT_LVAR
    (110, &[0, 0], 0),                   // MULT_INT_LVAR_BY_INT_VAR
    (111, &[0, 0], 0),                   // MULT_FLOAT_LVAR_BY_FLOAT_VAR
    (112, &[0, 0], 0),                   // DIV_INT_VAR_BY_INT_VAR
    (113, &[0, 0], 0),                   // DIV_FLOAT_VAR_BY_FLOAT_VAR
    (114, &[0, 0], 0),                   // DIV_INT_LVAR_BY_INT_LVAR
    (115, &[0, 0], 0),                   // DIV_FLOAT_LVAR_BY_FLOAT_LVAR
    (116, &[0, 0], 0),                   // DIV_INT_VAR_BY_INT_LVAR
    (117, &[0, 0], 0),                   // DIV_FLOAT_VAR_BY_FLOAT_LVAR
    (118, &[0, 0], 0),                   // DIV_INT_LVAR_BY_INT_VAR
    (119, &[0, 0], 0),                   // DIV_FLOAT_LVAR_BY_FLOAT_VAR
    (120, &[0, 0], 0),                   // ADD_TIMED_VAL_TO_FLOAT_VAR
    (121, &[0, 0], 0),                   // ADD_TIMED_VAL_TO_FLOAT_LVAR
    (122, &[0, 0], 0),                   // ADD_TIMED_FLOAT_VAR_TO_FLOAT_VAR
    (123, &[0, 0], 0),                   // ADD_TIMED_FLOAT_LVAR_TO_FLOAT_LVAR
    (124, &[0, 0], 0),                   // ADD_TIMED_FLOAT_LVAR_TO_FLOAT_VAR
    (125, &[0, 0], 0),                   // ADD_TIMED_FLOAT_VAR_TO_FLOAT_LVAR
    (126, &[0, 0], 0),                   // SUB_TIMED_VAL_FROM_FLOAT_VAR
    (127, &[0, 0], 0),                   // SUB_TIMED_VAL_FROM_FLOAT_LVAR
    (128, &[0, 0], 0),                   // SUB_TIMED_FLOAT_VAR_FROM_FLOAT_VAR
    (129, &[0, 0], 0),                   // SUB_TIMED_FLOAT_LVAR_FROM_FLOAT_LVAR
    (130, &[0, 0], 0),                   // SUB_TIMED_FLOAT_LVAR_FROM_FLOAT_VAR
    (131, &[0, 0], 0),                   // SUB_TIMED_FLOAT_VAR_FROM_FLOAT_LVAR
    (132, &[0, 0], 0),                   // SET_VAR_INT_TO_VAR_INT
    (133, &[0, 0], 0),                   // SET_LVAR_INT_TO_LVAR_INT
    (134, &[0, 0], 0),                   // SET_VAR_FLOAT_TO_VAR_FLOAT
    (135, &[0, 0], 0),                   // SET_LVAR_FLOAT_TO_LVAR_FLOAT
    (136, &[0, 0], 0),                   // SET_VAR_FLOAT_TO_LVAR_FLOAT
    (137, &[0, 0], 0),                   // SET_LVAR_FLOAT_TO_VAR_FLOAT
    (138, &[0, 0], 0),                   // SET_VAR_INT_TO_LVAR_INT
    (139, &[0, 0], 0),                   // SET_LVAR_INT_TO_VAR_INT
    (140, &[0, 0], 0),                   // CSET_VAR_INT_TO_VAR_FLOAT
    (141, &[0, 0], 0),                   // CSET_VAR_FLOAT_TO_VAR_INT
    (142, &[0, 0], 0),                   // CSET_LVAR_INT_TO_VAR_FLOAT
    (143, &[0, 0], 0),                   // CSET_LVAR_FLOAT_TO_VAR_INT
    (144, &[0, 0], 0),                   // CSET_VAR_INT_TO_LVAR_FLOAT
    (145, &[0, 0], 0),                   // CSET_VAR_FLOAT_TO_LVAR_INT
    (146, &[0, 0], 0),                   // CSET_LVAR_INT_TO_LVAR_FLOAT
    (147, &[0, 0], 0),                   // CSET_LVAR_FLOAT_TO_LVAR_INT
    (148, &[0, 0], 0),                   // ABS_VAR_INT
    (149, &[0, 0], 0),                   // ABS_LVAR_INT
    (150, &[0, 0], 0),                   // ABS_VAR_FLOAT
    (151, &[0, 0], 0),                   // ABS_LVAR_FLOAT
    (152, &[], 1),                       // GENERATE_RANDOM_FLOAT
    (153, &[], 1),                       // GENERATE_RANDOM_INT
    (154, &[0, 0, 0, 0, 0], 1),          // CREATE_CHAR
    (155, &[0], 0),                      // DELETE_CHAR
    (156, &[0, 0], 0),                   // CHAR_WANDER_DIR
    (157, &[], 0),                       // CHAR_WANDER_RANGE
    (158, &[0, 0, 0, 0], 0),             // CHAR_FOLLOW_PATH
    (159, &[0], 0),                      // CHAR_SET_IDLE
    (160, &[0], 3),                      // GET_CHAR_COORDINATES
    (161, &[0, 0, 0, 0], 0),             // SET_CHAR_COORDINATES
    (162, &[0], 0),                      // IS_CHAR_STILL_ALIVE
    (163, &[0, 0, 0, 0, 0, 0], 0),       // IS_CHAR_IN_AREA_2D
    (164, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CHAR_IN_AREA_3D
    (165, &[0, 0, 0, 0], 1),             // CREATE_CAR
    (166, &[0], 0),                      // DELETE_CAR
    (167, &[0, 0, 0, 0], 0),             // CAR_GOTO_COORDINATES
    (168, &[0], 0),                      // CAR_WANDER_RANDOMLY
    (169, &[0], 0),                      // CAR_SET_IDLE
    (170, &[0], 3),                      // GET_CAR_COORDINATES
    (171, &[0, 0, 0, 0], 0),             // SET_CAR_COORDINATES
    (172, &[0], 0),                      // IS_CAR_STILL_ALIVE
    (173, &[0, 0], 0),                   // SET_CAR_CRUISE_SPEED
    (174, &[0, 0], 0),                   // SET_CAR_DRIVING_STYLE
    (175, &[0, 0], 0),                   // SET_CAR_MISSION
    (176, &[0, 0, 0, 0, 0, 0], 0),       // IS_CAR_IN_AREA_2D
    (177, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CAR_IN_AREA_3D
    (178, &[], 0),                       // SPECIAL_0
    (179, &[], 0),                       // SPECIAL_1
    (180, &[], 0),                       // SPECIAL_2
    (181, &[], 0),                       // SPECIAL_3
    (182, &[], 0),                       // SPECIAL_4
    (183, &[], 0),                       // SPECIAL_5
    (184, &[], 0),                       // SPECIAL_6
    (185, &[], 0),                       // SPECIAL_7
    (186, &[1, 0, 0], 0),                // PRINT_BIG
    (187, &[1, 0, 0], 0),                // PRINT
    (188, &[1, 0, 0], 0),                // PRINT_NOW
    (189, &[1, 0, 0], 0),                // PRINT_SOON
    (190, &[], 0),                       // CLEAR_PRINTS
    (191, &[], 2),                       // GET_TIME_OF_DAY
    (192, &[0, 0], 0),                   // SET_TIME_OF_DAY
    (193, &[0, 0], 1),                   // GET_MINUTES_TO_TIME_OF_DAY
    (194, &[0, 0, 0, 0], 0),             // IS_POINT_ON_SCREEN
    (195, &[], 0),                       // DEBUG_ON
    (196, &[], 0),                       // DEBUG_OFF
    (197, &[], 0),                       // RETURN_TRUE
    (198, &[], 0),                       // RETURN_FALSE
    (199, &[], 0),                       // VAR_INT
    (200, &[], 0),                       // VAR_FLOAT
    (201, &[], 0),                       // LVAR_INT
    (202, &[], 0),                       // LVAR_FLOAT
    (203, &[], 0),                       // LBRACKET
    (204, &[], 0),                       // RBRACKET
    (205, &[], 0),                       // REPEAT
    (206, &[], 0),                       // ENDREPEAT
    (207, &[], 0),                       // IF
    (208, &[], 0),                       // IFNOT
    (209, &[], 0),                       // ELSE
    (210, &[], 0),                       // ENDIF
    (211, &[], 0),                       // WHILE
    (212, &[], 0),                       // WHILENOT
    (213, &[], 0),                       // ENDWHILE
    (214, &[0], 0),                      // ANDOR
    (215, &[0], 0),                      // LAUNCH_MISSION
    (216, &[], 0),                       // MISSION_HAS_FINISHED
    (217, &[0], 1),                      // STORE_CAR_CHAR_IS_IN
    (218, &[0], 1),                      // STORE_CAR_PLAYER_IS_IN
    (219, &[0, 0], 0),                   // IS_CHAR_IN_CAR
    (220, &[0, 0], 0),                   // IS_PLAYER_IN_CAR
    (221, &[0, 0], 0),                   // IS_CHAR_IN_MODEL
    (222, &[0, 0], 0),                   // IS_PLAYER_IN_MODEL
    (223, &[0], 0),                      // IS_CHAR_IN_ANY_CAR
    (224, &[0], 0),                      // IS_PLAYER_IN_ANY_CAR
    (225, &[0, 0], 0),                   // IS_BUTTON_PRESSED
    (226, &[0, 0], 1),                   // GET_PAD_STATE
    (227, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_ANY_MEANS_2D
    (228, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_ON_FOOT_2D
    (229, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_IN_CAR_2D
    (230, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_PLAYER_ANY_MEANS_2D
    (231, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_PLAYER_ON_FOOT_2D
    (232, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_PLAYER_IN_CAR_2D
    (233, &[0, 0, 0, 0, 0], 0),          // LOCATE_PLAYER_ANY_MEANS_CHAR_2D
    (234, &[0, 0, 0, 0, 0], 0),          // LOCATE_PLAYER_ON_FOOT_CHAR_2D
    (235, &[0, 0, 0, 0, 0], 0),          // LOCATE_PLAYER_IN_CAR_CHAR_2D
    (236, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_ANY_MEANS_2D
    (237, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_ON_FOOT_2D
    (238, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_IN_CAR_2D
    (239, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_CHAR_ANY_MEANS_2D
    (240, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_CHAR_ON_FOOT_2D
    (241, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_CHAR_IN_CAR_2D
    (242, &[0, 0, 0, 0, 0], 0),          // LOCATE_CHAR_ANY_MEANS_CHAR_2D
    (243, &[0, 0, 0, 0, 0], 0),          // LOCATE_CHAR_ON_FOOT_CHAR_2D
    (244, &[0, 0, 0, 0, 0], 0),          // LOCATE_CHAR_IN_CAR_CHAR_2D
    (245, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_PLAYER_ANY_MEANS_3D
    (246, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_PLAYER_ON_FOOT_3D
    (247, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_PLAYER_IN_CAR_3D
    (248, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_STOPPED_PLAYER_ANY_MEANS_3D
    (249, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_STOPPED_PLAYER_ON_FOOT_3D
    (250, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_STOPPED_PLAYER_IN_CAR_3D
    (251, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_ANY_MEANS_CHAR_3D
    (252, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_ON_FOOT_CHAR_3D
    (253, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_IN_CAR_CHAR_3D
    (254, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_CHAR_ANY_MEANS_3D
    (255, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_CHAR_ON_FOOT_3D
    (256, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_CHAR_IN_CAR_3D
    (257, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_STOPPED_CHAR_ANY_MEANS_3D
    (258, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_STOPPED_CHAR_ON_FOOT_3D
    (259, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_STOPPED_CHAR_IN_CAR_3D
    (260, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_ANY_MEANS_CHAR_3D
    (261, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_ON_FOOT_CHAR_3D
    (262, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_IN_CAR_CHAR_3D
    (263, &[0, 0, 0, 0], 1),             // CREATE_OBJECT
    (264, &[0], 0),                      // DELETE_OBJECT
    (265, &[0, 0], 0),                   // ADD_SCORE
    (266, &[0, 0], 0),                   // IS_SCORE_GREATER
    (267, &[0], 1),                      // STORE_SCORE
    (268, &[0, 0, 0, 0, 0], 0),          // GIVE_REMOTE_CONTROLLED_CAR_TO_PLAYER
    (269, &[0, 0], 0),                   // ALTER_WANTED_LEVEL
    (270, &[0, 0], 0),                   // ALTER_WANTED_LEVEL_NO_DROP
    (271, &[0, 0], 0),                   // IS_WANTED_LEVEL_GREATER
    (272, &[0], 0),                      // CLEAR_WANTED_LEVEL
    (273, &[0], 0),                      // SET_DEATHARREST_STATE
    (274, &[], 0),                       // HAS_DEATHARREST_BEEN_EXECUTED
    (275, &[0, 0, 0], 0),                // ADD_AMMO_TO_PLAYER
    (276, &[0, 0, 0], 0),                // ADD_AMMO_TO_CHAR
    (277, &[0, 0, 0], 0),                // ADD_AMMO_TO_CAR
    (278, &[0], 0),                      // IS_PLAYER_STILL_ALIVE
    (279, &[0], 0),                      // IS_PLAYER_DEAD
    (280, &[0], 0),                      // IS_CHAR_DEAD
    (281, &[0], 0),                      // IS_CAR_DEAD
    (282, &[0, 0], 0),                   // SET_CHAR_THREAT_SEARCH
    (283, &[0, 0], 0),                   // SET_CHAR_THREAT_REACTION
    (284, &[0], 0),                      // SET_CHAR_OBJ_NO_OBJ
    (285, &[0], 0),                      // ORDER_DRIVER_OUT_OF_CAR
    (286, &[0, 0], 0),                   // ORDER_CHAR_TO_DRIVE_CAR
    (287, &[0, 0, 0, 0], 0),             // ADD_PATROL_POINT
    (288, &[0, 0], 0),                   // IS_PLAYER_IN_GANGZONE
    (289, &[0, 1], 0),                   // IS_PLAYER_IN_ZONE
    (290, &[0], 0),                      // IS_PLAYER_PRESSING_HORN
    (291, &[0, 0], 0),                   // HAS_CHAR_SPOTTED_PLAYER
    (292, &[0, 0], 0),                   // ORDER_CHAR_TO_BACKDOOR
    (293, &[0, 0], 0),                   // ADD_CHAR_TO_GANG
    (294, &[0], 0),                      // IS_CHAR_OBJECTIVE_PASSED
    (295, &[0, 0], 0),                   // SET_CHAR_DRIVE_AGGRESSION
    (296, &[0, 0], 0),                   // SET_CHAR_MAX_DRIVESPEED
    (297, &[0, 0, 0], 1),                // CREATE_CHAR_INSIDE_CAR
    (298, &[0, 0, 0, 0], 0),             // WARP_PLAYER_FROM_CAR_TO_COORD
    (299, &[0], 0),                      // MAKE_CHAR_DO_NOTHING
    (300, &[0, 0], 0),                   // SET_CHAR_INVINCIBLE
    (301, &[0, 0], 0),                   // SET_PLAYER_INVINCIBLE
    (302, &[0, 0, 0], 0),                // SET_CHAR_GRAPHIC_TYPE
    (303, &[0, 0, 0], 0),                // SET_PLAYER_GRAPHIC_TYPE
    (304, &[0], 0),                      // HAS_PLAYER_BEEN_ARRESTED
    (305, &[0], 0),                      // STOP_CHAR_DRIVING
    (306, &[0], 0),                      // KILL_CHAR
    (307, &[0, 0], 0),                   // SET_FAVOURITE_CAR_MODEL_FOR_CHAR
    (308, &[0, 0], 0),                   // SET_CHAR_OCCUPATION
    (309, &[0, 0], 0),                   // CHANGE_CAR_LOCK
    (310, &[0, 0, 0, 0], 0),             // SHAKE_CAM_WITH_POINT
    (311, &[0, 0], 0),                   // IS_CAR_MODEL
    (312, &[0, 0], 0),                   // IS_CAR_REMAP
    (313, &[0], 0),                      // HAS_CAR_JUST_SUNK
    (314, &[0], 0),                      // SET_CAR_NO_COLLIDE
    (315, &[0, 0, 0, 0, 0, 0], 0),       // IS_CAR_DEAD_IN_AREA_2D
    (316, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CAR_DEAD_IN_AREA_3D
    (317, &[0, 0], 0),                   // IS_TRAILER_ATTACHED
    (318, &[0, 0], 0),                   // IS_CAR_ON_TRAILER
    (319, &[0, 0], 0),                   // HAS_CAR_GOT_WEAPON
    (320, &[0, 0], 0),                   // PARK
    (321, &[], 0),                       // HAS_PARK_FINISHED
    (322, &[0], 0),                      // KILL_ALL_PASSENGERS
    (323, &[0, 0], 0),                   // SET_CAR_BULLETPROOF
    (324, &[0, 0], 0),                   // SET_CAR_FLAMEPROOF
    (325, &[0, 0], 0),                   // SET_CAR_ROCKETPROOF
    (326, &[0], 0),                      // IS_CARBOMB_ACTIVE
    (327, &[0], 0),                      // GIVE_CAR_ALARM
    (328, &[0, 0], 0),                   // PUT_CAR_ON_TRAILER
    (329, &[0], 0),                      // IS_CAR_CRUSHED
    (330, &[0, 0, 0, 0], 1),             // CREATE_GANG_CAR
    (331, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 1), // CREATE_CAR_GENERATOR
    (332, &[0, 0], 0),                   // SWITCH_CAR_GENERATOR
    (333, &[1, 0, 0, 0], 0),             // ADD_PAGER_MESSAGE
    (334, &[0], 0),                      // DISPLAY_ONSCREEN_TIMER
    (335, &[0], 0),                      // CLEAR_ONSCREEN_TIMER
    (336, &[0, 0], 0),                   // DISPLAY_ONSCREEN_COUNTER
    (337, &[0], 0),                      // CLEAR_ONSCREEN_COUNTER
    (338, &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // SET_ZONE_CAR_INFO
    (339, &[0, 0], 0),                   // IS_CHAR_IN_GANG_ZONE
    (340, &[0, 1], 0),                   // IS_CHAR_IN_ZONE
    (341, &[1, 0, 0], 0),                // SET_CAR_DENSITY
    (342, &[1, 0, 0], 0),                // SET_PED_DENSITY
    (343, &[0, 0, 0], 0),                // POINT_CAMERA_AT_PLAYER
    (344, &[0, 0, 0], 0),                // POINT_CAMERA_AT_CAR
    (345, &[0, 0, 0], 0),                // POINT_CAMERA_AT_CHAR
    (346, &[], 0),                       // RESTORE_CAMERA
    (347, &[0, 0, 0], 0),                // SHAKE_PAD
    (348, &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // SET_ZONE_PED_INFO
    (349, &[0], 0),                      // SET_TIME_SCALE
    (350, &[0], 0),                      // IS_CAR_IN_AIR
    (351, &[0, 0, 0, 0, 0, 0], 0),       // SET_FIXED_CAMERA_POSITION
    (352, &[0, 0, 0, 0], 0),             // POINT_CAMERA_AT_POINT
    (353, &[0, 0, 0], 1),                // ADD_BLIP_FOR_CAR_OLD
    (354, &[0, 0, 0], 1),                // ADD_BLIP_FOR_CHAR_OLD
    (355, &[0, 0, 0], 1),                // ADD_BLIP_FOR_OBJECT_OLD
    (356, &[0], 0),                      // REMOVE_BLIP
    (357, &[0, 0], 0),                   // CHANGE_BLIP_COLOUR
    (358, &[0, 0], 0),                   // DIM_BLIP
    (359, &[0, 0, 0, 0, 0], 1),          // ADD_BLIP_FOR_COORD_OLD
    (360, &[0, 0], 0),                   // CHANGE_BLIP_SCALE
    (361, &[0, 0, 0], 0),                // SET_FADING_COLOUR
    (362, &[0, 0], 0),                   // DO_FADE
    (363, &[], 0),                       // GET_FADING_STATUS
    (364, &[0, 0, 0, 0], 0),             // ADD_HOSPITAL_RESTART
    (365, &[0, 0, 0, 0], 0),             // ADD_POLICE_RESTART
    (366, &[0, 0, 0, 0], 0),             // OVERRIDE_NEXT_RESTART
    (367, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // DRAW_SHADOW
    (368, &[0], 1),                      // GET_PLAYER_HEADING
    (369, &[0, 0], 0),                   // SET_PLAYER_HEADING
    (370, &[0], 1),                      // GET_CHAR_HEADING
    (371, &[0, 0], 0),                   // SET_CHAR_HEADING
    (372, &[0], 1),                      // GET_CAR_HEADING
    (373, &[0, 0], 0),                   // SET_CAR_HEADING
    (374, &[0], 1),                      // GET_OBJECT_HEADING
    (375, &[0, 0], 0),                   // SET_OBJECT_HEADING
    (376, &[0, 0], 0),                   // IS_PLAYER_TOUCHING_OBJECT
    (377, &[0, 0], 0),                   // IS_CHAR_TOUCHING_OBJECT
    (378, &[0, 0, 0], 0),                // SET_PLAYER_AMMO
    (379, &[0, 0, 0], 0),                // SET_CHAR_AMMO
    (380, &[0, 0, 0], 0),                // SET_CAR_AMMO
    (381, &[1], 0),                      // LOAD_CAMERA_SPLINE
    (382, &[0], 0),                      // MOVE_CAMERA_ALONG_SPLINE
    (383, &[], 1),                       // GET_CAMERA_POSITION_ALONG_SPLINE
    (384, &[0], 0),                      // DECLARE_MISSION_FLAG
    (385, &[0, 0], 0),                   // DECLARE_MISSION_FLAG_FOR_CONTACT
    (386, &[0, 0], 0),                   // DECLARE_BASE_BRIEF_ID_FOR_CONTACT
    (387, &[0, 0], 0),                   // IS_PLAYER_HEALTH_GREATER
    (388, &[0, 0], 0),                   // IS_CHAR_HEALTH_GREATER
    (389, &[0, 0], 0),                   // IS_CAR_HEALTH_GREATER
    (390, &[0], 1),                      // ADD_BLIP_FOR_CAR
    (391, &[0], 1),                      // ADD_BLIP_FOR_CHAR
    (392, &[0], 1),                      // ADD_BLIP_FOR_OBJECT
    (393, &[0, 0, 0], 1),                // ADD_BLIP_FOR_CONTACT_POINT
    (394, &[0, 0, 0], 1),                // ADD_BLIP_FOR_COORD
    (395, &[0, 0], 0),                   // CHANGE_BLIP_DISPLAY
    (396, &[0, 0, 0, 0], 0),             // ADD_ONE_OFF_SOUND
    (397, &[0, 0, 0, 0], 1),             // ADD_CONTINUOUS_SOUND
    (398, &[0], 0),                      // REMOVE_SOUND
    (399, &[0], 0),                      // IS_CAR_STUCK_ON_ROOF
    (400, &[0], 0),                      // ADD_UPSIDEDOWN_CAR_CHECK
    (401, &[0], 0),                      // REMOVE_UPSIDEDOWN_CAR_CHECK
    (402, &[0], 0),                      // SET_CHAR_OBJ_WAIT_ON_FOOT
    (403, &[0], 0),                      // SET_CHAR_OBJ_FLEE_ON_FOOT_TILL_SAFE
    (404, &[0, 0, 0, 0], 0),             // SET_CHAR_OBJ_GUARD_SPOT
    (405, &[0, 0, 0, 0, 0], 0),          // SET_CHAR_OBJ_GUARD_AREA
    (406, &[0], 0),                      // SET_CHAR_OBJ_WAIT_IN_CAR
    (407, &[0, 0, 0, 0, 0, 0], 0),       // IS_PLAYER_IN_AREA_ON_FOOT_2D
    (408, &[0, 0, 0, 0, 0, 0], 0),       // IS_PLAYER_IN_AREA_IN_CAR_2D
    (409, &[0, 0, 0, 0, 0, 0], 0),       // IS_PLAYER_STOPPED_IN_AREA_2D
    (410, &[0, 0, 0, 0, 0, 0], 0),       // IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_2D
    (411, &[0, 0, 0, 0, 0, 0], 0),       // IS_PLAYER_STOPPED_IN_AREA_IN_CAR_2D
    (412, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_IN_AREA_ON_FOOT_3D
    (413, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_IN_AREA_IN_CAR_3D
    (414, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_STOPPED_IN_AREA_3D
    (415, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_STOPPED_IN_AREA_ON_FOOT_3D
    (416, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_STOPPED_IN_AREA_IN_CAR_3D
    (417, &[0, 0, 0, 0, 0, 0], 0),       // IS_CHAR_IN_AREA_ON_FOOT_2D
    (418, &[0, 0, 0, 0, 0, 0], 0),       // IS_CHAR_IN_AREA_IN_CAR_2D
    (419, &[0, 0, 0, 0, 0, 0], 0),       // IS_CHAR_STOPPED_IN_AREA_2D
    (420, &[0, 0, 0, 0, 0, 0], 0),       // IS_CHAR_STOPPED_IN_AREA_ON_FOOT_2D
    (421, &[0, 0, 0, 0, 0, 0], 0),       // IS_CHAR_STOPPED_IN_AREA_IN_CAR_2D
    (422, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CHAR_IN_AREA_ON_FOOT_3D
    (423, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CHAR_IN_AREA_IN_CAR_3D
    (424, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CHAR_STOPPED_IN_AREA_3D
    (425, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CHAR_STOPPED_IN_AREA_ON_FOOT_3D
    (426, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CHAR_STOPPED_IN_AREA_IN_CAR_3D
    (427, &[0, 0, 0, 0, 0, 0], 0),       // IS_CAR_STOPPED_IN_AREA_2D
    (428, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_CAR_STOPPED_IN_AREA_3D
    (429, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CAR_2D
    (430, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_CAR_2D
    (431, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_CAR_3D
    (432, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // LOCATE_STOPPED_CAR_3D
    (433, &[0, 0, 0], 0),                // GIVE_WEAPON_TO_PLAYER
    (434, &[0, 0, 0], 0),                // GIVE_WEAPON_TO_CHAR
    (435, &[0, 0, 0], 0),                // GIVE_WEAPON_TO_CAR
    (436, &[0, 0], 0),                   // SET_PLAYER_CONTROL
    (437, &[0], 0),                      // FORCE_WEATHER
    (438, &[0], 0),                      // FORCE_WEATHER_NOW
    (439, &[], 0),                       // RELEASE_WEATHER
    (440, &[0, 0], 0),                   // SET_CURRENT_PLAYER_WEAPON
    (441, &[0, 0], 0),                   // SET_CURRENT_CHAR_WEAPON
    (442, &[0, 0], 0),                   // SET_CURRENT_CAR_WEAPON
    (443, &[0], 3),                      // GET_OBJECT_COORDINATES
    (444, &[0, 0, 0, 0], 0),             // SET_OBJECT_COORDINATES
    (445, &[], 1),                       // GET_GAME_TIMER
    (446, &[0, 0, 0, 0], 0),             // TURN_CHAR_TO_FACE_COORD
    (447, &[0, 0, 0, 0], 0),             // TURN_PLAYER_TO_FACE_COORD
    (448, &[0], 1),                      // STORE_WANTED_LEVEL
    (449, &[0], 0),                      // IS_CAR_STOPPED
    (450, &[0], 0),                      // MARK_CHAR_AS_NO_LONGER_NEEDED
    (451, &[0], 0),                      // MARK_CAR_AS_NO_LONGER_NEEDED
    (452, &[0], 0),                      // MARK_OBJECT_AS_NO_LONGER_NEEDED
    (453, &[0], 0),                      // DONT_REMOVE_CHAR
    (454, &[0], 0),                      // DONT_REMOVE_CAR
    (455, &[0], 0),                      // DONT_REMOVE_OBJECT
    (456, &[0, 0, 0, 0], 1),             // CREATE_CHAR_AS_PASSENGER
    (457, &[0, 0], 0),                   // SET_CHAR_OBJ_KILL_CHAR_ON_FOOT
    (458, &[0, 0], 0),                   // SET_CHAR_OBJ_KILL_PLAYER_ON_FOOT
    (459, &[0, 0], 0),                   // SET_CHAR_OBJ_KILL_CHAR_ANY_MEANS
    (460, &[0, 0], 0),                   // SET_CHAR_OBJ_KILL_PLAYER_ANY_MEANS
    (461, &[0, 0], 0),                   // SET_CHAR_OBJ_FLEE_CHAR_ON_FOOT_TILL_SAFE
    (462, &[0, 0], 0),                   // SET_CHAR_OBJ_FLEE_PLAYER_ON_FOOT_TILL_SAFE
    (463, &[0, 0], 0),                   // SET_CHAR_OBJ_FLEE_CHAR_ON_FOOT_ALWAYS
    (464, &[0, 0], 0),                   // SET_CHAR_OBJ_FLEE_PLAYER_ON_FOOT_ALWAYS
    (465, &[0, 0], 0),                   // SET_CHAR_OBJ_GOTO_CHAR_ON_FOOT
    (466, &[0, 0], 0),                   // SET_CHAR_OBJ_GOTO_PLAYER_ON_FOOT
    (467, &[0, 0], 0),                   // SET_CHAR_OBJ_LEAVE_CAR
    (468, &[0, 0], 0),                   // SET_CHAR_OBJ_ENTER_CAR_AS_PASSENGER
    (469, &[0, 0], 0),                   // SET_CHAR_OBJ_ENTER_CAR_AS_DRIVER
    (470, &[0], 0),                      // SET_CHAR_OBJ_FOLLOW_CAR_IN_CAR
    (471, &[0], 0),                      // SET_CHAR_OBJ_FIRE_AT_OBJECT_FROM_VEHICLE
    (472, &[0, 0], 0),                   // SET_CHAR_OBJ_DESTROY_OBJECT
    (473, &[0, 0], 0),                   // SET_CHAR_OBJ_DESTROY_CAR
    (474, &[0, 0, 0, 0, 0], 0),          // SET_CHAR_OBJ_GOTO_AREA_ON_FOOT
    (475, &[0], 0),                      // SET_CHAR_OBJ_GOTO_AREA_IN_CAR
    (476, &[0], 0),                      // SET_CHAR_OBJ_FOLLOW_CAR_ON_FOOT_WITH_OFFSET
    (477, &[0], 0),                      // SET_CHAR_OBJ_GUARD_ATTACK
    (478, &[0, 0], 0),                   // SET_CHAR_AS_LEADER
    (479, &[0, 0], 0),                   // SET_PLAYER_AS_LEADER
    (480, &[0], 0),                      // LEAVE_GROUP
    (481, &[0, 0, 0], 0),                // SET_CHAR_OBJ_FOLLOW_ROUTE
    (482, &[0, 0, 0, 0], 0),             // ADD_ROUTE_POINT
    (483, &[1, 0, 0, 0], 0),             // PRINT_WITH_NUMBER_BIG
    (484, &[1, 0, 0, 0], 0),             // PRINT_WITH_NUMBER
    (485, &[1, 0, 0, 0], 0),             // PRINT_WITH_NUMBER_NOW
    (486, &[1, 0, 0, 0], 0),             // PRINT_WITH_NUMBER_SOON
    (487, &[0, 0, 0, 0, 0, 0], 0),       // SWITCH_ROADS_ON
    (488, &[0, 0, 0, 0, 0, 0], 0),       // SWITCH_ROADS_OFF
    (489, &[0], 1),                      // GET_NUMBER_OF_PASSENGERS
    (490, &[0], 1),                      // GET_MAXIMUM_NUMBER_OF_PASSENGERS
    (491, &[0], 0),                      // SET_CAR_DENSITY_MULTIPLIER
    (492, &[0, 0], 0),                   // SET_CAR_HEAVY
    (493, &[0], 0),                      // CLEAR_CHAR_THREAT_SEARCH
    (494, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // ACTIVATE_CRANE
    (495, &[0, 0], 0),                   // DEACTIVATE_CRANE
    (496, &[0], 0),                      // SET_MAX_WANTED_LEVEL
    (497, &[], 0),                       // SAVE_VAR_INT
    (498, &[], 0),                       // SAVE_VAR_FLOAT
    (499, &[0], 0),                      // IS_CAR_IN_AIR_PROPER
    (500, &[0], 0),                      // IS_CAR_UPSIDEDOWN
    (501, &[0], 1),                      // GET_PLAYER_CHAR
    (502, &[], 0),                       // CANCEL_OVERRIDE_RESTART
    (503, &[0, 0], 0),                   // SET_POLICE_IGNORE_PLAYER
    (504, &[1, 0, 0, 0, 0], 0),          // ADD_PAGER_MESSAGE_WITH_NUMBER
    (505, &[1, 0, 0, 0, 0, 0, 0, 0, 0], 0), // START_KILL_FRENZY
    (506, &[], 1),                       // READ_KILL_FRENZY_STATUS
    (507, &[0], 1),                      // SQRT
    (508, &[0, 0, 0, 0, 0], 0),          // LOCATE_PLAYER_ANY_MEANS_CAR_2D
    (509, &[0, 0, 0, 0, 0], 0),          // LOCATE_PLAYER_ON_FOOT_CAR_2D
    (510, &[0, 0, 0, 0, 0], 0),          // LOCATE_PLAYER_IN_CAR_CAR_2D
    (511, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_ANY_MEANS_CAR_3D
    (512, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_ON_FOOT_CAR_3D
    (513, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_PLAYER_IN_CAR_CAR_3D
    (514, &[0, 0, 0, 0, 0], 0),          // LOCATE_CHAR_ANY_MEANS_CAR_2D
    (515, &[0, 0, 0, 0, 0], 0),          // LOCATE_CHAR_ON_FOOT_CAR_2D
    (516, &[0, 0, 0, 0, 0], 0),          // LOCATE_CHAR_IN_CAR_CAR_2D
    (517, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_ANY_MEANS_CAR_3D
    (518, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_ON_FOOT_CAR_3D
    (519, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_CHAR_IN_CAR_CAR_3D
    (520, &[0, 0], 1),                   // GENERATE_RANDOM_FLOAT_IN_RANGE
    (521, &[0, 0], 1),                   // GENERATE_RANDOM_INT_IN_RANGE
    (522, &[0, 0], 0),                   // LOCK_CAR_DOORS
    (523, &[0], 0),                      // EXPLODE_CAR
    (524, &[0, 0, 0, 0], 0),             // ADD_EXPLOSION
    (525, &[0], 0),                      // IS_CAR_UPRIGHT
    (526, &[0, 0], 0),                   // TURN_CHAR_TO_FACE_CHAR
    (527, &[0, 0], 0),                   // TURN_CHAR_TO_FACE_PLAYER
    (528, &[0, 0], 0),                   // TURN_PLAYER_TO_FACE_CHAR
    (529, &[0, 0, 0], 0),                // SET_CHAR_OBJ_GOTO_COORD_ON_FOOT
    (530, &[0], 0),                      // SET_CHAR_OBJ_GOTO_COORD_IN_CAR
    (531, &[0, 0, 0, 0, 0], 1),          // CREATE_PICKUP
    (532, &[0], 0),                      // HAS_PICKUP_BEEN_COLLECTED
    (533, &[0], 0),                      // REMOVE_PICKUP
    (534, &[0, 0], 0),                   // SET_TAXI_LIGHTS
    (535, &[1, 0, 0], 0),                // PRINT_BIG_Q
    (536, &[1, 0, 0, 0], 0),             // PRINT_WITH_NUMBER_BIG_Q
    (537, &[0, 0, 0, 0, 0, 0, 0], 1),    // SET_GARAGE
    (538, &[0, 0, 0, 0, 0, 0, 0, 0], 1), // SET_GARAGE_WITH_CAR_MODEL
    (539, &[0, 0], 0),                   // SET_TARGET_CAR_FOR_MISSION_GARAGE
    (540, &[0], 0),                      // IS_CAR_IN_MISSION_GARAGE
    (541, &[0], 0),                      // SET_FREE_BOMBS
    (542, &[0, 0, 0, 0, 0, 0, 0], 0),    // SET_POWERPOINT
    (543, &[0], 0),                      // SET_ALL_TAXI_LIGHTS
    (544, &[0], 0),                      // IS_CAR_ARMED_WITH_ANY_BOMB
    (545, &[0, 0], 0),                   // APPLY_BRAKES_TO_PLAYERS_CAR
    (546, &[0, 0], 0),                   // SET_PLAYER_HEALTH
    (547, &[0, 0], 0),                   // SET_CHAR_HEALTH
    (548, &[0, 0], 0),                   // SET_CAR_HEALTH
    (549, &[0], 1),                      // GET_PLAYER_HEALTH
    (550, &[0], 1),                      // GET_CHAR_HEALTH
    (551, &[0], 1),                      // GET_CAR_HEALTH
    (552, &[0, 0], 0),                   // IS_CAR_ARMED_WITH_BOMB
    (553, &[0, 0, 0], 0),                // CHANGE_CAR_COLOUR
    (554, &[0, 0, 0, 0, 0, 0], 0),       // SWITCH_PED_ROADS_ON
    (555, &[0, 0, 0, 0, 0, 0], 0),       // SWITCH_PED_ROADS_OFF
    (556, &[0, 0], 0),                   // CHAR_LOOK_AT_CHAR_ALWAYS
    (557, &[0, 0], 0),                   // CHAR_LOOK_AT_PLAYER_ALWAYS
    (558, &[0, 0], 0),                   // PLAYER_LOOK_AT_CHAR_ALWAYS
    (559, &[0], 0),                      // STOP_CHAR_LOOKING
    (560, &[0], 0),                      // STOP_PLAYER_LOOKING
    (561, &[0], 0),                      // SWITCH_HELICOPTER
    (562, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // SET_GANG_ATTITUDE
    (563, &[0, 0, 0], 0),                // SET_GANG_GANG_ATTITUDE
    (564, &[0, 0, 0], 0),                // SET_GANG_PLAYER_ATTITUDE
    (565, &[0, 0, 0], 0),                // SET_GANG_PED_MODELS
    (566, &[0, 0], 0),                   // SET_GANG_CAR_MODEL
    (567, &[0, 0, 0], 0),                // SET_GANG_WEAPONS
    (568, &[0, 0, 0, 0, 0], 0),          // SET_CHAR_OBJ_RUN_TO_AREA
    (569, &[0, 0, 0], 0),                // SET_CHAR_OBJ_RUN_TO_COORD
    (570, &[0, 0], 0),                   // IS_PLAYER_TOUCHING_OBJECT_ON_FOOT
    (571, &[0, 0], 0),                   // IS_CHAR_TOUCHING_OBJECT_ON_FOOT
    (572, &[0, 1], 0),                   // LOAD_SPECIAL_CHARACTER
    (573, &[0], 0),                      // HAS_SPECIAL_CHARACTER_LOADED
    (574, &[0, 0], 0),                   // FLASH_CAR
    (575, &[0, 0], 0),                   // FLASH_CHAR
    (576, &[0, 0], 0),                   // FLASH_OBJECT
    (577, &[0], 0),                      // IS_PLAYER_IN_REMOTE_MODE
    (578, &[0, 0], 0),                   // ARM_CAR_WITH_BOMB
    (579, &[0, 0], 0),                   // SET_CHAR_PERSONALITY
    (580, &[0, 0, 0], 0),                // SET_CUTSCENE_OFFSET
    (581, &[0, 0], 0),                   // SET_ANIM_GROUP_FOR_CHAR
    (582, &[0, 0], 0),                   // SET_ANIM_GROUP_FOR_PLAYER
    (583, &[0], 0),                      // REQUEST_MODEL
    (584, &[0], 0),                      // HAS_MODEL_LOADED
    (585, &[0], 0),                      // MARK_MODEL_AS_NO_LONGER_NEEDED
    (586, &[0, 0], 1),                   // GRAB_PHONE
    (587, &[0, 1], 0),                   // SET_REPEATED_PHONE_MESSAGE
    (588, &[0, 1], 0),                   // SET_PHONE_MESSAGE
    (589, &[0], 0),                      // HAS_PHONE_DISPLAYED_MESSAGE
    (590, &[0], 0),                      // TURN_PHONE_OFF
    (591, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // DRAW_CORONA
    (592, &[0, 0, 0, 0, 0, 0], 0),       // DRAW_LIGHT
    (593, &[], 0),                       // STORE_WEATHER
    (594, &[], 0),                       // RESTORE_WEATHER
    (595, &[], 0),                       // STORE_CLOCK
    (596, &[], 0),                       // RESTORE_CLOCK
    (597, &[0, 0, 0, 0], 0),             // RESTART_CRITICAL_MISSION
    (598, &[0], 0),                      // IS_PLAYER_PLAYING
    (599, &[0], 0),                      // SET_COLL_OBJ_NO_OBJ
    (600, &[0], 0),                      // SET_COLL_OBJ_WAIT_ON_FOOT
    (601, &[0], 0),                      // SET_COLL_OBJ_FLEE_ON_FOOT_TILL_SAFE
    (602, &[0, 0, 0, 0], 0),             // SET_COLL_OBJ_GUARD_SPOT
    (603, &[0, 0, 0, 0, 0], 0),          // SET_COLL_OBJ_GUARD_AREA
    (604, &[0], 0),                      // SET_COLL_OBJ_WAIT_IN_CAR
    (605, &[0, 0], 0),                   // SET_COLL_OBJ_KILL_CHAR_ON_FOOT
    (606, &[0, 0], 0),                   // SET_COLL_OBJ_KILL_PLAYER_ON_FOOT
    (607, &[0, 0], 0),                   // SET_COLL_OBJ_KILL_CHAR_ANY_MEANS
    (608, &[0, 0], 0),                   // SET_COLL_OBJ_KILL_PLAYER_ANY_MEANS
    (609, &[0, 0], 0),                   // SET_COLL_OBJ_FLEE_CHAR_ON_FOOT_TILL_SAFE
    (610, &[0, 0], 0),                   // SET_COLL_OBJ_FLEE_PLAYER_ON_FOOT_TILL_SAFE
    (611, &[0, 0], 0),                   // SET_COLL_OBJ_FLEE_CHAR_ON_FOOT_ALWAYS
    (612, &[0, 0], 0),                   // SET_COLL_OBJ_FLEE_PLAYER_ON_FOOT_ALWAYS
    (613, &[0, 0], 0),                   // SET_COLL_OBJ_GOTO_CHAR_ON_FOOT
    (614, &[0, 0], 0),                   // SET_COLL_OBJ_GOTO_PLAYER_ON_FOOT
    (615, &[0], 0),                      // SET_COLL_OBJ_LEAVE_CAR
    (616, &[0, 0], 0),                   // SET_COLL_OBJ_ENTER_CAR_AS_PASSENGER
    (617, &[0, 0], 0),                   // SET_COLL_OBJ_ENTER_CAR_AS_DRIVER
    (618, &[0], 0),                      // SET_COLL_OBJ_FOLLOW_CAR_IN_CAR
    (619, &[0], 0),                      // SET_COLL_OBJ_FIRE_AT_OBJECT_FROM_VEHICLE
    (620, &[0, 0], 0),                   // SET_COLL_OBJ_DESTROY_OBJECT
    (621, &[0, 0], 0),                   // SET_COLL_OBJ_DESTROY_CAR
    (622, &[0, 0, 0, 0, 0], 0),          // SET_COLL_OBJ_GOTO_AREA_ON_FOOT
    (623, &[0], 0),                      // SET_COLL_OBJ_GOTO_AREA_IN_CAR
    (624, &[0], 0),                      // SET_COLL_OBJ_FOLLOW_CAR_ON_FOOT_WITH_OFFSET
    (625, &[0], 0),                      // SET_COLL_OBJ_GUARD_ATTACK
    (626, &[0, 0, 0], 0),                // SET_COLL_OBJ_FOLLOW_ROUTE
    (627, &[0, 0, 0], 0),                // SET_COLL_OBJ_GOTO_COORD_ON_FOOT
    (628, &[0], 0),                      // SET_COLL_OBJ_GOTO_COORD_IN_CAR
    (629, &[0, 0, 0, 0, 0], 0),          // SET_COLL_OBJ_RUN_TO_AREA
    (630, &[0, 0, 0], 0),                // SET_COLL_OBJ_RUN_TO_COORD
    (631, &[0, 0, 0], 1),                // ADD_PEDS_IN_AREA_TO_COLL
    (632, &[0], 1),                      // ADD_PEDS_IN_VEHICLE_TO_COLL
    (633, &[0], 0),                      // CLEAR_COLL
    (634, &[0], 0),                      // IS_COLL_IN_CARS
    (635, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_COLL_ANY_MEANS_2D
    (636, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_COLL_ON_FOOT_2D
    (637, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_COLL_IN_CAR_2D
    (638, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_COLL_ANY_MEANS_2D
    (639, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_COLL_ON_FOOT_2D
    (640, &[0, 0, 0, 0, 0, 0], 0),       // LOCATE_STOPPED_COLL_IN_CAR_2D
    (641, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_ANY_MEANS_CHAR_2D
    (642, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_ON_FOOT_CHAR_2D
    (643, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_IN_CAR_CHAR_2D
    (644, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_ANY_MEANS_CAR_2D
    (645, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_ON_FOOT_CAR_2D
    (646, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_IN_CAR_CAR_2D
    (647, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_ANY_MEANS_PLAYER_2D
    (648, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_ON_FOOT_PLAYER_2D
    (649, &[0, 0, 0, 0, 0], 0),          // LOCATE_COLL_IN_CAR_PLAYER_2D
    (650, &[0, 0, 0, 0, 0, 0], 0),       // IS_COLL_IN_AREA_2D
    (651, &[0, 0, 0, 0, 0, 0], 0),       // IS_COLL_IN_AREA_ON_FOOT_2D
    (652, &[0, 0, 0, 0, 0, 0], 0),       // IS_COLL_IN_AREA_IN_CAR_2D
    (653, &[0, 0, 0, 0, 0, 0], 0),       // IS_COLL_STOPPED_IN_AREA_2D
    (654, &[0, 0, 0, 0, 0, 0], 0),       // IS_COLL_STOPPED_IN_AREA_ON_FOOT_2D
    (655, &[0, 0, 0, 0, 0, 0], 0),       // IS_COLL_STOPPED_IN_AREA_IN_CAR_2D
    (656, &[0], 1),                      // GET_NUMBER_OF_PEDS_IN_COLL
    (657, &[0, 0], 0),                   // SET_CHAR_HEED_THREATS
    (658, &[0, 0], 0),                   // SET_PLAYER_HEED_THREATS
    (659, &[], 1),                       // GET_CONTROLLER_MODE
    (660, &[0, 0], 0),                   // SET_CAN_RESPRAY_CAR
    (661, &[0], 0),                      // IS_TAXI
    (662, &[0], 0),                      // UNLOAD_SPECIAL_CHARACTER
    (663, &[], 0),                       // RESET_NUM_OF_MODELS_KILLED_BY_PLAYER
    (664, &[0], 1),                      // GET_NUM_OF_MODELS_KILLED_BY_PLAYER
    (665, &[0], 0),                      // ACTIVATE_GARAGE
    (666, &[0], 0),                      // SWITCH_TAXI_TIMER
    (667, &[0, 0, 0, 0], 1),             // CREATE_OBJECT_NO_OFFSET
    (668, &[0], 0),                      // IS_BOAT
    (669, &[0, 0, 0, 0, 0], 0),          // SET_CHAR_OBJ_GOTO_AREA_ANY_MEANS
    (670, &[0, 0, 0, 0, 0], 0),          // SET_COLL_OBJ_GOTO_AREA_ANY_MEANS
    (671, &[0], 0),                      // IS_PLAYER_STOPPED
    (672, &[0], 0),                      // IS_CHAR_STOPPED
    (673, &[0, 0], 0),                   // MESSAGE_WAIT
    (674, &[0, 0, 0, 0, 0], 0),          // ADD_PARTICLE_EFFECT
    (675, &[0], 0),                      // SWITCH_WIDESCREEN
    (676, &[0, 0], 1),                   // ADD_SPRITE_BLIP_FOR_CAR
    (677, &[0, 0], 1),                   // ADD_SPRITE_BLIP_FOR_CHAR
    (678, &[0, 0], 1),                   // ADD_SPRITE_BLIP_FOR_OBJECT
    (679, &[0, 0, 0, 0], 1),             // ADD_SPRITE_BLIP_FOR_CONTACT_POINT
    (680, &[0, 0, 0, 0], 1),             // ADD_SPRITE_BLIP_FOR_COORD
    (681, &[0, 0], 0),                   // SET_CHAR_ONLY_DAMAGED_BY_PLAYER
    (682, &[0, 0], 0),                   // SET_CAR_ONLY_DAMAGED_BY_PLAYER
    (683, &[0, 0, 0, 0, 0, 0], 0),       // SET_CHAR_PROOFS
    (684, &[0, 0, 0, 0, 0, 0], 0),       // SET_CAR_PROOFS
    (685, &[0, 0, 0, 0, 0, 0, 0], 0),    // IS_PLAYER_IN_ANGLED_AREA_2D
    (686, &[0, 0, 0, 0, 0, 0, 0], 0),    // IS_PLAYER_IN_ANGLED_AREA_ON_FOOT_2D
    (687, &[0, 0, 0, 0, 0, 0, 0], 0),    // IS_PLAYER_IN_ANGLED_AREA_IN_CAR_2D
    (688, &[0, 0, 0, 0, 0, 0, 0], 0),    // IS_PLAYER_STOPPED_IN_ANGLED_AREA_2D
    (689, &[0, 0, 0, 0, 0, 0, 0], 0),    // IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_2D
    (690, &[0, 0, 0, 0, 0, 0, 0], 0),    // IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_2D
    (691, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_IN_ANGLED_AREA_3D
    (692, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_IN_ANGLED_AREA_ON_FOOT_3D
    (693, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_IN_ANGLED_AREA_IN_CAR_3D
    (694, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_STOPPED_IN_ANGLED_AREA_3D
    (695, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_STOPPED_IN_ANGLED_AREA_ON_FOOT_3D
    (696, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_PLAYER_STOPPED_IN_ANGLED_AREA_IN_CAR_3D
    (697, &[0], 0),                      // DEACTIVATE_GARAGE
    (698, &[0], 1),                      // GET_NUMBER_OF_CARS_COLLECTED_BY_GARAGE
    (699, &[0, 0], 0),                   // HAS_CAR_BEEN_TAKEN_TO_GARAGE
    (700, &[0], 0),                      // SET_SWAT_REQUIRED
    (701, &[0], 0),                      // SET_FBI_REQUIRED
    (702, &[0], 0),                      // SET_ARMY_REQUIRED
    (703, &[0], 0),                      // IS_CAR_IN_WATER
    (704, &[0, 0, 0], 3),                // GET_CLOSEST_CHAR_NODE
    (705, &[0, 0, 0], 3),                // GET_CLOSEST_CAR_NODE
    (706, &[0, 0, 0, 0], 0),             // CAR_GOTO_COORDINATES_ACCURATE
    (707, &[0], 0),                      // START_PACMAN_RACE
    (708, &[], 0),                       // START_PACMAN_RECORD
    (709, &[], 1),                       // GET_NUMBER_OF_POWER_PILLS_EATEN
    (710, &[], 0),                       // CLEAR_PACMAN
    (711, &[0, 0, 0, 0, 0], 0),          // START_PACMAN_SCRAMBLE
    (712, &[], 1),                       // GET_NUMBER_OF_POWER_PILLS_CARRIED
    (713, &[], 0),                       // CLEAR_NUMBER_OF_POWER_PILLS_CARRIED
    (714, &[0], 0),                      // IS_CAR_ON_SCREEN
    (715, &[0], 0),                      // IS_CHAR_ON_SCREEN
    (716, &[0], 0),                      // IS_OBJECT_ON_SCREEN
    (717, &[0, 0], 0),                   // GOSUB_FILE
    (718, &[0, 0, 0], 1),                // GET_GROUND_Z_FOR_3D_COORD
    (719, &[0, 0, 0], 1),                // START_SCRIPT_FIRE
    (720, &[0], 0),                      // IS_SCRIPT_FIRE_EXTINGUISHED
    (721, &[0], 0),                      // REMOVE_SCRIPT_FIRE
    (722, &[0, 0], 0),                   // SET_COMEDY_CONTROLS
    (723, &[0, 0, 0, 0], 0),             // BOAT_GOTO_COORDS
    (724, &[0], 0),                      // BOAT_STOP
    (725, &[0, 0, 0, 0, 0, 0], 0),       // IS_PLAYER_SHOOTING_IN_AREA
    (726, &[0, 0, 0, 0, 0, 0], 0),       // IS_CHAR_SHOOTING_IN_AREA
    (727, &[0, 0], 0),                   // IS_CURRENT_PLAYER_WEAPON
    (728, &[0, 0], 0),                   // IS_CURRENT_CHAR_WEAPON
    (729, &[], 0),                       // CLEAR_NUMBER_OF_POWER_PILLS_EATEN
    (730, &[0, 0, 0], 0),                // ADD_POWER_PILL
    (731, &[0, 0], 0),                   // SET_BOAT_CRUISE_SPEED
    (732, &[0, 0, 0, 0], 1),             // GET_RANDOM_CHAR_IN_AREA
    (733, &[1], 1),                      // GET_RANDOM_CHAR_IN_ZONE
    (734, &[0], 0),                      // IS_PLAYER_IN_TAXI
    (735, &[0], 0),                      // IS_PLAYER_SHOOTING
    (736, &[0], 0),                      // IS_CHAR_SHOOTING
    (737, &[0, 0, 0, 0], 1),             // CREATE_MONEY_PICKUP
    (738, &[0, 0], 0),                   // SET_CHAR_ACCURACY
    (739, &[0], 1),                      // GET_CAR_SPEED
    (740, &[1], 0),                      // LOAD_CUTSCENE
    (741, &[0], 1),                      // CREATE_CUTSCENE_OBJECT
    (742, &[0, 1], 0),                   // SET_CUTSCENE_ANIM
    (743, &[], 0),                       // START_CUTSCENE
    (744, &[], 1),                       // GET_CUTSCENE_TIME
    (745, &[], 0),                       // HAS_CUTSCENE_FINISHED
    (746, &[], 0),                       // CLEAR_CUTSCENE
    (747, &[], 0),                       // RESTORE_CAMERA_JUMPCUT
    (748, &[0, 0, 0], 0),                // CREATE_COLLECTABLE1
    (749, &[0], 0),                      // SET_COLLECTABLE1_TOTAL
    (750, &[0, 0, 0, 0, 0, 0], 0),       // IS_PROJECTILE_IN_AREA
    (751, &[0, 0, 0, 0, 0, 0], 0),       // DESTROY_PROJECTILES_IN_AREA
    (752, &[0, 0, 0], 0),                // DROP_MINE
    (753, &[0, 0, 0], 0),                // DROP_NAUTICAL_MINE
    (754, &[0, 0], 0),                   // IS_CHAR_MODEL
    (755, &[0, 1], 0),                   // LOAD_SPECIAL_MODEL
    (756, &[0, 0], 1),                   // CREATE_CUTSCENE_HEAD
    (757, &[0, 1], 0),                   // SET_CUTSCENE_HEAD_ANIM
    (758, &[0], 1),                      // SIN
    (759, &[0], 1),                      // COS
    (760, &[0], 1),                      // GET_CAR_FORWARD_X
    (761, &[0], 1),                      // GET_CAR_FORWARD_Y
    (762, &[0, 0], 0),                   // CHANGE_GARAGE_TYPE
    (763, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // ACTIVATE_CRUSHER_CRANE
    (764, &[1, 0, 0, 0, 0], 0),          // PRINT_WITH_2_NUMBERS
    (765, &[1, 0, 0, 0, 0], 0),          // PRINT_WITH_2_NUMBERS_NOW
    (766, &[1, 0, 0, 0, 0], 0),          // PRINT_WITH_2_NUMBERS_SOON
    (767, &[1, 0, 0, 0, 0, 0], 0),       // PRINT_WITH_3_NUMBERS
    (768, &[1, 0, 0, 0, 0, 0], 0),       // PRINT_WITH_3_NUMBERS_NOW
    (769, &[1, 0, 0, 0, 0, 0], 0),       // PRINT_WITH_3_NUMBERS_SOON
    (770, &[1, 0, 0, 0, 0, 0, 0], 0),    // PRINT_WITH_4_NUMBERS
    (771, &[1, 0, 0, 0, 0, 0, 0], 0),    // PRINT_WITH_4_NUMBERS_NOW
    (772, &[1, 0, 0, 0, 0, 0, 0], 0),    // PRINT_WITH_4_NUMBERS_SOON
    (773, &[1, 0, 0, 0, 0, 0, 0, 0], 0), // PRINT_WITH_5_NUMBERS
    (774, &[1, 0, 0, 0, 0, 0, 0, 0], 0), // PRINT_WITH_5_NUMBERS_NOW
    (775, &[1, 0, 0, 0, 0, 0, 0, 0], 0), // PRINT_WITH_5_NUMBERS_SOON
    (776, &[1, 0, 0, 0, 0, 0, 0, 0, 0], 0), // PRINT_WITH_6_NUMBERS
    (777, &[1, 0, 0, 0, 0, 0, 0, 0, 0], 0), // PRINT_WITH_6_NUMBERS_NOW
    (778, &[1, 0, 0, 0, 0, 0, 0, 0, 0], 0), // PRINT_WITH_6_NUMBERS_SOON
    (779, &[0, 0, 0], 0),                // SET_CHAR_OBJ_FOLLOW_CHAR_IN_FORMATION
    (780, &[0], 0),                      // PLAYER_MADE_PROGRESS
    (781, &[0], 0),                      // SET_PROGRESS_TOTAL
    (782, &[0], 0),                      // REGISTER_JUMP_DISTANCE
    (783, &[0], 0),                      // REGISTER_JUMP_HEIGHT
    (784, &[0], 0),                      // REGISTER_JUMP_FLIPS
    (785, &[0], 0),                      // REGISTER_JUMP_SPINS
    (786, &[0], 0),                      // REGISTER_JUMP_STUNT
    (787, &[], 0),                       // REGISTER_UNIQUE_JUMP_FOUND
    (788, &[0], 0),                      // SET_UNIQUE_JUMPS_TOTAL
    (789, &[], 0),                       // REGISTER_PASSENGER_DROPPED_OFF_TAXI
    (790, &[0], 0),                      // REGISTER_MONEY_MADE_TAXI
    (791, &[], 0),                       // REGISTER_MISSION_GIVEN
    (792, &[1], 0),                      // REGISTER_MISSION_PASSED
    (793, &[0, 0], 0),                   // SET_CHAR_RUNNING
    (794, &[], 0),                       // REMOVE_ALL_SCRIPT_FIRES
    (795, &[0, 0], 0),                   // IS_FIRST_CAR_COLOUR
    (796, &[0, 0], 0),                   // IS_SECOND_CAR_COLOUR
    (797, &[0, 0], 0),                   // HAS_CHAR_BEEN_DAMAGED_BY_WEAPON
    (798, &[0, 0], 0),                   // HAS_CAR_BEEN_DAMAGED_BY_WEAPON
    (799, &[0, 0], 0),                   // IS_CHAR_IN_CHARS_GROUP
    (800, &[0, 0], 0),                   // IS_CHAR_IN_PLAYERS_GROUP
    (801, &[0], 0),                      // EXPLODE_CHAR_HEAD
    (802, &[0], 0),                      // EXPLODE_PLAYER_HEAD
    (803, &[0, 0], 0),                   // ANCHOR_BOAT
    (804, &[1, 0, 0], 0),                // SET_ZONE_GROUP
    (805, &[0], 1),                      // START_CAR_FIRE
    (806, &[0], 1),                      // START_CHAR_FIRE
    (807, &[0, 0, 0, 0, 0], 1),          // GET_RANDOM_CAR_OF_TYPE_IN_AREA
    (808, &[1, 0], 1),                   // GET_RANDOM_CAR_OF_TYPE_IN_ZONE
    (809, &[0], 0),                      // HAS_RESPRAY_HAPPENED
    (810, &[0], 0),                      // SET_CAMERA_ZOOM
    (811, &[0, 0, 0, 0, 0, 0], 1),       // CREATE_PICKUP_WITH_AMMO
    (812, &[0, 0], 0),                   // SET_CAR_RAM_CAR
    (813, &[0, 0], 0),                   // SET_CAR_BLOCK_CAR
    (814, &[0], 0),                      // SET_CHAR_OBJ_CATCH_TRAIN
    (815, &[0], 0),                      // SET_COLL_OBJ_CATCH_TRAIN
    (816, &[0, 0], 0),                   // SET_PLAYER_NEVER_GETS_TIRED
    (817, &[0, 0], 0),                   // SET_PLAYER_FAST_RELOAD
    (818, &[0, 0], 0),                   // SET_CHAR_BLEEDING
    (819, &[0, 0], 0),                   // SET_CAR_FUNNY_SUSPENSION
    (820, &[0, 0], 0),                   // SET_CAR_BIG_WHEELS
    (821, &[0], 0),                      // SET_FREE_RESPRAYS
    (822, &[0, 0], 0),                   // SET_PLAYER_VISIBLE
    (823, &[0, 0], 0),                   // SET_CHAR_VISIBLE
    (824, &[0, 0], 0),                   // SET_CAR_VISIBLE
    (825, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // IS_AREA_OCCUPIED
    (826, &[], 0),                       // START_DRUG_RUN
    (827, &[], 0),                       // HAS_DRUG_RUN_BEEN_COMPLETED
    (828, &[], 0),                       // HAS_DRUG_PLANE_BEEN_SHOT_DOWN
    (829, &[0], 0),                      // SAVE_PLAYER_FROM_FIRES
    (830, &[0, 0, 1], 0),                // DISPLAY_TEXT
    (831, &[0, 0], 0),                   // SET_TEXT_SCALE
    (832, &[0, 0, 0, 0], 0),             // SET_TEXT_COLOUR
    (833, &[0], 0),                      // SET_TEXT_JUSTIFY
    (834, &[0], 0),                      // SET_TEXT_CENTRE
    (835, &[0], 0),                      // SET_TEXT_WRAPX
    (836, &[0], 0),                      // SET_TEXT_CENTRE_SIZE
    (837, &[0], 0),                      // SET_TEXT_BACKGROUND
    (838, &[0, 0, 0, 0], 0),             // SET_TEXT_BACKGROUND_COLOUR
    (839, &[0], 0),                      // SET_TEXT_BACKGROUND_ONLY_TEXT
    (840, &[0], 0),                      // SET_TEXT_PROPORTIONAL
    (841, &[0], 0),                      // SET_TEXT_FONT
    (842, &[], 0),                       // INDUSTRIAL_PASSED
    (843, &[], 0),                       // COMMERCIAL_PASSED
    (844, &[], 0),                       // SUBURBAN_PASSED
    (845, &[0, 0, 0, 0], 0),             // ROTATE_OBJECT
    (846, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // SLIDE_OBJECT
    (847, &[0], 0),                      // REMOVE_CHAR_ELEGANTLY
    (848, &[0, 0], 0),                   // SET_CHAR_STAY_IN_SAME_PLACE
    (849, &[], 0),                       // IS_NASTY_GAME
    (850, &[0, 1], 0),                   // UNDRESS_CHAR
    (851, &[0], 0),                      // DRESS_CHAR
    (852, &[0], 0),                      // START_CHASE_SCENE
    (853, &[], 0),                       // STOP_CHASE_SCENE
    (854, &[0, 0, 0, 0, 0, 0, 0], 0),    // IS_EXPLOSION_IN_AREA
    (855, &[0, 1], 0),                   // IS_EXPLOSION_IN_ZONE
    (856, &[], 0),                       // START_DRUG_DROP_OFF
    (857, &[], 0),                       // HAS_DROP_OFF_PLANE_BEEN_SHOT_DOWN
    (858, &[], 3),                       // FIND_DROP_OFF_PLANE_COORDINATES
    (859, &[0, 0, 0], 1),                // CREATE_FLOATING_PACKAGE
    (860, &[0, 0, 0, 0, 0], 0),          // PLACE_OBJECT_RELATIVE_TO_CAR
    (861, &[0], 0),                      // MAKE_OBJECT_TARGETTABLE
    (862, &[0, 0], 0),                   // ADD_ARMOUR_TO_PLAYER
    (863, &[0, 0], 0),                   // ADD_ARMOUR_TO_CHAR
    (864, &[0], 0),                      // OPEN_GARAGE
    (865, &[0], 0),                      // CLOSE_GARAGE
    (866, &[0, 0, 0, 0], 0),             // WARP_CHAR_FROM_CAR_TO_COORD
    (867, &[0, 0, 0, 0, 0, 0], 0),       // SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE
    (868, &[0, 0], 0),                   // HAS_CHAR_SPOTTED_CHAR
    (869, &[0], 0),                      // SET_CHAR_OBJ_HAIL_TAXI
    (870, &[0], 0),                      // HAS_OBJECT_BEEN_DAMAGED
    (871, &[1, 0, 0, 0, 0, 0, 0, 0, 0], 0), // START_KILL_FRENZY_HEADSHOT
    (872, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // ACTIVATE_MILITARY_CRANE
    (873, &[0, 0], 0),                   // WARP_PLAYER_INTO_CAR
    (874, &[0, 0], 0),                   // WARP_CHAR_INTO_CAR
    (875, &[0], 0),                      // SWITCH_CAR_RADIO
    (876, &[0], 0),                      // SET_AUDIO_STREAM
    (877, &[1, 0, 0, 0, 0], 0),          // PRINT_WITH_2_NUMBERS_BIG
    (878, &[1, 0, 0, 0, 0, 0], 0),       // PRINT_WITH_3_NUMBERS_BIG
    (879, &[1, 0, 0, 0, 0, 0, 0], 0),    // PRINT_WITH_4_NUMBERS_BIG
    (880, &[1, 0, 0, 0, 0, 0, 0, 0], 0), // PRINT_WITH_5_NUMBERS_BIG
    (881, &[1, 0, 0, 0, 0, 0, 0, 0, 0], 0), // PRINT_WITH_6_NUMBERS_BIG
    (882, &[0, 0, 0], 0),                // SET_CHAR_WAIT_STATE
    (883, &[], 0),                       // SET_CAMERA_BEHIND_PLAYER
    (884, &[0], 0),                      // SET_MOTION_BLUR
    (885, &[1, 1, 0, 0], 0),             // PRINT_STRING_IN_STRING
    (886, &[0, 0, 0], 1),                // CREATE_RANDOM_CHAR
    (887, &[0], 0),                      // SET_CHAR_OBJ_STEAL_ANY_CAR
    (888, &[0, 1, 1], 0),                // SET_2_REPEATED_PHONE_MESSAGES
    (889, &[0, 1, 1], 0),                // SET_2_PHONE_MESSAGES
    (890, &[0, 1, 1, 1], 0),             // SET_3_REPEATED_PHONE_MESSAGES
    (891, &[0, 1, 1, 1], 0),             // SET_3_PHONE_MESSAGES
    (892, &[0, 1, 1, 1, 1], 0),          // SET_4_REPEATED_PHONE_MESSAGES
    (893, &[0, 1, 1, 1, 1], 0),          // SET_4_PHONE_MESSAGES
    (894, &[0, 0, 0, 0, 0, 0], 0),       // IS_SNIPER_BULLET_IN_AREA
    (895, &[], 0),                       // GIVE_PLAYER_DETONATOR
    (896, &[0], 0),                      // SET_COLL_OBJ_STEAL_ANY_CAR
    (897, &[0, 0, 0, 0], 0),             // SET_OBJECT_VELOCITY
    (898, &[0, 0], 0),                   // SET_OBJECT_COLLISION
    (899, &[0], 0),                      // IS_ICECREAM_JINGLE_ON
    (900, &[1, 1, 0, 0], 0),             // PRINT_STRING_IN_STRING_NOW
    (901, &[1, 1, 0, 0], 0),             // PRINT_STRING_IN_STRING_SOON
    (902, &[0, 1, 1, 1, 1, 1], 0),       // SET_5_REPEATED_PHONE_MESSAGES
    (903, &[0, 1, 1, 1, 1, 1], 0),       // SET_5_PHONE_MESSAGES
    (904, &[0, 1, 1, 1, 1, 1, 1], 0),    // SET_6_REPEATED_PHONE_MESSAGES
    (905, &[0, 1, 1, 1, 1, 1, 1], 0),    // SET_6_PHONE_MESSAGES
    (906, &[0, 0, 0, 0, 0, 0], 0),       // IS_POINT_OBSCURED_BY_A_MISSION_ENTITY
    (907, &[], 0),                       // LOAD_ALL_MODELS_NOW
    (908, &[0, 0, 0, 0], 0),             // ADD_TO_OBJECT_VELOCITY
    (909, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // DRAW_SPRITE
    (910, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // DRAW_RECT
    (911, &[0, 1], 0),                   // LOAD_SPRITE
    (912, &[1], 0),                      // LOAD_TEXTURE_DICTIONARY
    (913, &[], 0),                       // REMOVE_TEXTURE_DICTIONARY
    (914, &[0, 0], 0),                   // SET_OBJECT_DYNAMIC
    (915, &[0, 0], 0),                   // SET_CHAR_ANIM_SPEED
    (916, &[0], 0),                      // PLAY_MISSION_PASSED_TUNE
    (917, &[0, 0, 0, 0, 0], 0),          // CLEAR_AREA
    (918, &[0], 0),                      // FREEZE_ONSCREEN_TIMER
    (919, &[0, 0], 0),                   // SWITCH_CAR_SIREN
    (920, &[0, 0, 0, 0, 0, 0, 0], 0),    // SWITCH_PED_ROADS_ON_ANGLED
    (921, &[0, 0, 0, 0, 0, 0, 0], 0),    // SWITCH_PED_ROADS_OFF_ANGLED
    (922, &[0, 0, 0, 0, 0, 0, 0], 0),    // SWITCH_ROADS_ON_ANGLED
    (923, &[0, 0, 0, 0, 0, 0, 0], 0),    // SWITCH_ROADS_OFF_ANGLED
    (924, &[0, 0], 0),                   // SET_CAR_WATERTIGHT
    (925, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 0), // ADD_MOVING_PARTICLE_EFFECT
    (926, &[0, 0], 0),                   // SET_CHAR_CANT_BE_DRAGGED_OUT
    (927, &[0, 0, 0], 0),                // TURN_CAR_TO_FACE_COORD
    (928, &[0, 0, 0], 0),                // IS_CRANE_LIFTING_CAR
    (929, &[0, 0, 0, 0], 0),             // DRAW_SPHERE
    (930, &[0, 0], 0),                   // SET_CAR_STATUS
    (931, &[0], 0),                      // IS_CHAR_MALE
    (932, &[1], 0),                      // SCRIPT_NAME
    (933, &[0, 0, 0], 0),                // CHANGE_GARAGE_TYPE_WITH_CAR_MODEL
    (934, &[], 3),                       // FIND_DRUG_PLANE_COORDINATES
    (935, &[0], 0),                      // SAVE_INT_TO_DEBUG_FILE
    (936, &[0], 0),                      // SAVE_FLOAT_TO_DEBUG_FILE
    (937, &[], 0),                       // SAVE_NEWLINE_TO_DEBUG_FILE
    (938, &[0, 0, 0], 0),                // POLICE_RADIO_MESSAGE
    (939, &[0, 0], 0),                   // SET_CAR_STRONG
    (940, &[0], 0),                      // REMOVE_ROUTE
    (941, &[0], 0),                      // SWITCH_RUBBISH
    (942, &[0, 0, 0, 0, 0, 0], 0),       // REMOVE_PARTICLE_EFFECTS_IN_AREA
    (943, &[0], 0),                      // SWITCH_STREAMING
    (944, &[0], 0),                      // IS_GARAGE_OPEN
    (945, &[0], 0),                      // IS_GARAGE_CLOSED
    (946, &[], 0),                       // START_CATALINA_HELI
    (947, &[], 0),                       // CATALINA_HELI_TAKE_OFF
    (948, &[], 0),                       // REMOVE_CATALINA_HELI
    (949, &[], 0),                       // HAS_CATALINA_HELI_BEEN_SHOT_DOWN
    (950, &[0, 0, 0, 0, 0, 0], 0),       // SWAP_NEAREST_BUILDING_MODEL
    (951, &[0], 0),                      // SWITCH_WORLD_PROCESSING
    (952, &[0], 0),                      // REMOVE_ALL_PLAYER_WEAPONS
    (953, &[], 1),                       // GRAB_CATALINA_HELI
    (954, &[0, 0, 0, 0, 0, 0], 0),       // CLEAR_AREA_OF_CARS
    (955, &[0], 0),                      // SET_ROTATING_GARAGE_DOOR
    (956, &[0, 0, 0, 0], 1),             // ADD_SPHERE
    (957, &[0], 0),                      // REMOVE_SPHERE
    (958, &[], 0),                       // CATALINA_HELI_FLY_AWAY
    (959, &[0, 0], 0),                   // SET_EVERYONE_IGNORE_PLAYER
    (960, &[0], 1),                      // STORE_CAR_CHAR_IS_IN_NO_SAVE
    (961, &[0], 1),                      // STORE_CAR_PLAYER_IS_IN_NO_SAVE
    (962, &[0], 0),                      // IS_PHONE_DISPLAYING_MESSAGE
    (963, &[0, 1], 0),                   // DISPLAY_ONSCREEN_TIMER_WITH_STRING
    (964, &[0, 0, 1], 0),                // DISPLAY_ONSCREEN_COUNTER_WITH_STRING
    (965, &[0, 0, 0, 0], 0),             // CREATE_RANDOM_CAR_FOR_CAR_PARK
    (966, &[0], 0),                      // IS_COLLISION_IN_MEMORY
    (967, &[0], 0),                      // SET_WANTED_MULTIPLIER
    (968, &[], 0),                       // SET_CAMERA_IN_FRONT_OF_PLAYER
    (969, &[0], 0),                      // IS_CAR_VISIBLY_DAMAGED
    (970, &[0], 0),                      // DOES_OBJECT_EXIST
    (971, &[0, 0, 0], 0),                // LOAD_SCENE
    (972, &[0, 0, 0], 0),                // ADD_STUCK_CAR_CHECK
    (973, &[0], 0),                      // REMOVE_STUCK_CAR_CHECK
    (974, &[0], 0),                      // IS_CAR_STUCK
    (975, &[1], 0),                      // LOAD_MISSION_AUDIO
    (976, &[], 0),                       // HAS_MISSION_AUDIO_LOADED
    (977, &[], 0),                       // PLAY_MISSION_AUDIO
    (978, &[], 0),                       // HAS_MISSION_AUDIO_FINISHED
    (979, &[0, 0, 0], 4),                // GET_CLOSEST_CAR_NODE_WITH_HEADING
    (980, &[0, 0], 0),                   // HAS_IMPORT_GARAGE_SLOT_BEEN_FILLED
    (981, &[1], 0),                      // CLEAR_THIS_PRINT
    (982, &[1], 0),                      // CLEAR_THIS_BIG_PRINT
    (983, &[0, 0, 0], 0),                // SET_MISSION_AUDIO_POSITION
    (984, &[], 0),                       // ACTIVATE_SAVE_MENU
    (985, &[], 0),                       // HAS_SAVE_GAME_FINISHED
    (986, &[0], 0),                      // NO_SPECIAL_CAMERA_FOR_THIS_GARAGE
    (987, &[0, 0, 0], 1),                // ADD_BLIP_FOR_PICKUP_OLD
    (988, &[0], 1),                      // ADD_BLIP_FOR_PICKUP
    (989, &[0, 0], 1),                   // ADD_SPRITE_BLIP_FOR_PICKUP
    (990, &[0], 0),                      // SET_PED_DENSITY_MULTIPLIER
    (991, &[0], 0),                      // FORCE_RANDOM_PED_TYPE
    (992, &[0], 0),                      // SET_TEXT_DRAW_BEFORE_FADE
    (993, &[], 1),                       // GET_COLLECTABLE1S_COLLECTED
    (994, &[0], 0),                      // REGISTER_EL_BURRO_TIME
    (995, &[0], 0),                      // SET_SPRITES_DRAW_BEFORE_FADE
    (996, &[0], 0),                      // SET_TEXT_RIGHT_JUSTIFY
    (997, &[1], 0),                      // PRINT_HELP
    (998, &[], 0),                       // CLEAR_HELP
    (999, &[0], 0),                      // FLASH_HUD_OBJECT
    (1000, &[0], 0),                     // FLASH_RADAR_BLIP
    (1001, &[0], 0),                     // IS_CHAR_IN_CONTROL
    (1002, &[0], 0),                     // SET_GENERATE_CARS_AROUND_CAMERA
    (1003, &[], 0),                      // CLEAR_SMALL_PRINTS
    (1004, &[], 0),                      // HAS_MILITARY_CRANE_COLLECTED_ALL_CARS
    (1005, &[0, 0], 0),                  // SET_UPSIDEDOWN_CAR_NOT_DAMAGED
    (1006, &[0], 0),                     // CAN_PLAYER_START_MISSION
    (1007, &[0], 0),                     // MAKE_PLAYER_SAFE_FOR_CUTSCENE
    (1008, &[0], 0),                     // USE_TEXT_COMMANDS
    (1009, &[0, 0], 0),                  // SET_THREAT_FOR_PED_TYPE
    (1010, &[0, 0], 0),                  // CLEAR_THREAT_FOR_PED_TYPE
    (1011, &[0], 2),                     // GET_CAR_COLOURS
    (1012, &[0], 0),                     // SET_ALL_CARS_CAN_BE_DAMAGED
    (1013, &[0, 0], 0),                  // SET_CAR_CAN_BE_DAMAGED
    (1014, &[0], 0),                     // MAKE_PLAYER_UNSAFE
    (1015, &[0], 0),                     // LOAD_COLLISION
    (1016, &[], 1),                      // GET_BODY_CAST_HEALTH
    (1017, &[0, 0, 0], 0),               // SET_CHARS_CHATTING
    (1018, &[0], 0),                     // MAKE_PLAYER_SAFE
    (1019, &[0, 0], 0),                  // SET_CAR_STAYS_IN_CURRENT_LEVEL
    (1020, &[0, 0], 0),                  // SET_CHAR_STAYS_IN_CURRENT_LEVEL
    (1021, &[0], 0),                     // REGISTER_4X4_ONE_TIME
    (1022, &[0], 0),                     // REGISTER_4X4_TWO_TIME
    (1023, &[0], 0),                     // REGISTER_4X4_THREE_TIME
    (1024, &[0], 0),                     // REGISTER_4X4_MAYHEM_TIME
    (1025, &[], 0),                      // REGISTER_LIFE_SAVED
    (1026, &[], 0),                      // REGISTER_CRIMINAL_CAUGHT
    (1027, &[0], 0),                     // REGISTER_AMBULANCE_LEVEL
    (1028, &[], 0),                      // REGISTER_FIRE_EXTINGUISHED
    (1029, &[0], 0),                     // TURN_PHONE_ON
    (1030, &[0], 0),                     // REGISTER_LONGEST_DODO_FLIGHT
    (1031, &[0], 0),                     // REGISTER_DEFUSE_BOMB_TIME
    (1032, &[0], 0),                     // SET_TOTAL_NUMBER_OF_KILL_FRENZIES
    (1033, &[], 0),                      // BLOW_UP_RC_BUGGY
    (1034, &[0], 0),                     // REMOVE_CAR_FROM_CHASE
    (1035, &[], 0),                      // IS_FRENCH_GAME
    (1036, &[], 0),                      // IS_GERMAN_GAME
    (1037, &[], 0),                      // CLEAR_MISSION_AUDIO
    (1038, &[0], 0),                     // SET_FADE_IN_AFTER_NEXT_ARREST
    (1039, &[0], 0),                     // SET_FADE_IN_AFTER_NEXT_DEATH
    (1040, &[0, 0], 0),                  // SET_GANG_PED_MODEL_PREFERENCE
    (1041, &[0, 0], 0),                  // SET_CHAR_USE_PEDNODE_SEEK
    (1042, &[0, 0], 0),                  // SWITCH_VEHICLE_WEAPONS
    (1043, &[0, 0], 0),                  // SET_GET_OUT_OF_JAIL_FREE
    (1044, &[0, 0], 0),                  // SET_FREE_HEALTH_CARE
    (1045, &[0, 0], 0),                  // IS_CAR_DOOR_CLOSED
    (1046, &[], 0),                      // LOAD_AND_LAUNCH_MISSION
    (1047, &[0], 0),                     // LOAD_AND_LAUNCH_MISSION_INTERNAL
    (1048, &[0, 0], 0),                  // SET_OBJECT_DRAW_LAST
    (1049, &[0, 0], 1),                  // GET_AMMO_IN_PLAYER_WEAPON
    (1050, &[0, 0], 1),                  // GET_AMMO_IN_CHAR_WEAPON
    (1051, &[], 0),                      // REGISTER_KILL_FRENZY_PASSED
    (1052, &[0, 0], 0),                  // SET_CHAR_SAY
    (1053, &[0], 0),                     // SET_NEAR_CLIP
    (1054, &[0, 0], 0),                  // SET_RADIO_CHANNEL
    (1055, &[0], 0),                     // OVERRIDE_HOSPITAL_LEVEL
    (1056, &[0], 0),                     // OVERRIDE_POLICE_STATION_LEVEL
    (1057, &[0], 0),                     // FORCE_RAIN
    (1058, &[0, 0], 0),                  // DOES_GARAGE_CONTAIN_CAR
    (1059, &[0, 0], 0),                  // SET_CAR_TRACTION
    (1060, &[], 0),                      // ARE_MEASUREMENTS_IN_METRES
    (1061, &[0], 1),                     // CONVERT_METRES_TO_FEET
    (1062, &[0, 0, 0, 0, 0, 0], 0),      // MARK_ROADS_BETWEEN_LEVELS
    (1063, &[0, 0, 0, 0, 0, 0], 0),      // MARK_PED_ROADS_BETWEEN_LEVELS
    (1064, &[0, 0], 0),                  // SET_CAR_AVOID_LEVEL_TRANSITIONS
    (1065, &[0, 0], 0),                  // SET_CHAR_AVOID_LEVEL_TRANSITIONS
    (1066, &[0, 0], 0),                  // IS_THREAT_FOR_PED_TYPE
    (1067, &[0, 0, 0, 0, 0, 0], 0),      // CLEAR_AREA_OF_CHARS
    (1068, &[0], 0),                     // SET_TOTAL_NUMBER_OF_MISSIONS
    (1069, &[0], 1),                     // CONVERT_METRES_TO_FEET_INT
    (1070, &[0, 0], 0),                  // REGISTER_FASTEST_TIME
    (1071, &[0, 0], 0),                  // REGISTER_HIGHEST_SCORE
    (1072, &[0, 0, 0], 0),               // WARP_CHAR_INTO_CAR_AS_PASSENGER
    (1073, &[0, 0], 0),                  // IS_CAR_PASSENGER_SEAT_FREE
    (1074, &[0, 0], 1),                  // GET_CHAR_IN_CAR_PASSENGER_SEAT
    (1075, &[0, 0], 0),                  // SET_CHAR_IS_CHRIS_CRIMINAL
    (1076, &[], 0),                      // START_CREDITS
    (1077, &[], 0),                      // STOP_CREDITS
    (1078, &[], 0),                      // ARE_CREDITS_FINISHED
    (1079, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // CREATE_SINGLE_PARTICLE
    (1080, &[0, 0], 0),                  // SET_CHAR_IGNORE_LEVEL_TRANSITIONS
    (1081, &[0], 1),                     // GET_CHASE_CAR
    (1082, &[], 0),                      // START_BOAT_FOAM_ANIMATION
    (1083, &[0], 0),                     // UPDATE_BOAT_FOAM_ANIMATION
    (1084, &[0], 0),                     // SET_MUSIC_DOES_FADE
    (1085, &[0], 0),                     // SET_INTRO_IS_PLAYING
    (1086, &[0, 0], 0),                  // SET_PLAYER_HOOKER
    (1087, &[], 0),                      // PLAY_END_OF_GAME_TUNE
    (1088, &[], 0),                      // STOP_END_OF_GAME_TUNE
    (1089, &[0], 1),                     // GET_CAR_MODEL
    (1090, &[0, 0], 0),                  // IS_PLAYER_SITTING_IN_CAR
    (1091, &[0], 0),                     // IS_PLAYER_SITTING_IN_ANY_CAR
    (1092, &[0, 0], 0),                  // SET_SCRIPT_FIRE_AUDIO
    (1093, &[], 0),                      // ARE_ANY_CAR_CHEATS_ACTIVATED
    (1094, &[0, 0], 0),                  // SET_CHAR_SUFFERS_CRITICAL_HITS
    (1095, &[0], 0),                     // IS_PLAYER_LIFTING_A_PHONE
    (1096, &[0, 0], 0),                  // IS_CHAR_SITTING_IN_CAR
    (1097, &[0], 0),                     // IS_CHAR_SITTING_IN_ANY_CAR
    (1098, &[0], 0),                     // IS_PLAYER_ON_FOOT
    (1099, &[0], 0),                     // IS_CHAR_ON_FOOT
    (1100, &[0], 0),                     // LOAD_COLLISION_WITH_SCREEN
    (1101, &[1], 0),                     // LOAD_SPLASH_SCREEN
    (1102, &[0, 0], 0),                  // SET_CAR_IGNORE_LEVEL_TRANSITIONS
    (1103, &[0, 0], 0),                  // MAKE_CRAIGS_CAR_A_BIT_STRONGER
    (1104, &[0], 0),                     // SET_JAMES_CAR_ON_PATH_TO_PLAYER
    (1105, &[], 0),                      // LOAD_END_OF_GAME_TUNE
    (1106, &[], 0),                      // ENABLE_PLAYER_CONTROL_CAMERA
    (1107, &[0, 0, 0, 0], 0),            // SET_OBJECT_ROTATION
    (1108, &[], 3),                      // GET_DEBUG_CAMERA_COORDINATES
    (1109, &[], 3),                      // GET_DEBUG_CAMERA_FRONT_VECTOR
    (1110, &[0], 0),                     // IS_PLAYER_TARGETTING_ANY_CHAR
    (1111, &[0, 0], 0),                  // IS_PLAYER_TARGETTING_CHAR
    (1112, &[0, 0], 0),                  // IS_PLAYER_TARGETTING_OBJECT
    (1113, &[1], 0),                     // TERMINATE_ALL_SCRIPTS_WITH_THIS_NAME
    (1114, &[0, 0, 1, 0], 0),            // DISPLAY_TEXT_WITH_NUMBER
    (1115, &[0, 0, 1, 0, 0], 0),         // DISPLAY_TEXT_WITH_2_NUMBERS
    (1116, &[], 0),                      // FAIL_CURRENT_MISSION
    (1117, &[0, 0, 0, 0, 0], 1),         // GET_CLOSEST_OBJECT_OF_TYPE
    (1118, &[0, 0, 0, 0, 0], 0),         // PLACE_OBJECT_RELATIVE_TO_OBJECT
    (1119, &[0], 0),                     // SET_ALL_OCCUPANTS_OF_CAR_LEAVE_CAR
    (1120, &[0, 0], 0),                  // SET_INTERPOLATION_PARAMETERS
    (1121, &[0, 0, 0, 0, 0], 4),         // GET_CLOSEST_CAR_NODE_WITH_HEADING_TOWARDS_POINT
    (1122, &[0, 0, 0, 0, 0], 4),         // GET_CLOSEST_CAR_NODE_WITH_HEADING_AWAY_POINT
    (1123, &[], 3),                      // GET_DEBUG_CAMERA_POINT_AT
    (1124, &[0, 0, 0, 0, 0, 0, 0, 0], 0), // ATTACH_CHAR_TO_CAR
    (1125, &[0], 0),                     // DETACH_CHAR_FROM_CAR
    (1126, &[0, 0], 0),                  // SET_CAR_CHANGE_LANE
    (1127, &[0], 0),                     // CLEAR_CHAR_LAST_WEAPON_DAMAGE
    (1128, &[0], 0),                     // CLEAR_CAR_LAST_WEAPON_DAMAGE
    (1129, &[0, 0, 0, 0, 0, 0, 0, 0, 0], 1), // GET_RANDOM_COP_IN_AREA
    (1130, &[1, 0, 0, 0, 0, 0], 1),      // GET_RANDOM_COP_IN_ZONE
    (1131, &[0, 0], 0),                  // SET_CHAR_OBJ_FLEE_CAR
    (1132, &[0], 1),                     // GET_DRIVER_OF_CAR
    (1133, &[0], 1),                     // GET_NUMBER_OF_FOLLOWERS
    (1134, &[0, 0, 0, 0, 0, 0], 0),      // GIVE_REMOTE_CONTROLLED_MODEL_TO_PLAYER
    (1135, &[0], 1),                     // GET_CURRENT_PLAYER_WEAPON
    (1136, &[0], 1),                     // GET_CURRENT_CHAR_WEAPON
    (1137, &[0, 0, 0, 0, 0], 0),         // LOCATE_CHAR_ANY_MEANS_OBJECT_2D
    (1138, &[0, 0, 0, 0, 0], 0),         // LOCATE_CHAR_ON_FOOT_OBJECT_2D
    (1139, &[0, 0, 0, 0, 0], 0),         // LOCATE_CHAR_IN_CAR_OBJECT_2D
    (1140, &[0, 0, 0, 0, 0, 0], 0),      // LOCATE_CHAR_ANY_MEANS_OBJECT_3D
    (1141, &[0, 0, 0, 0, 0, 0], 0),      // LOCATE_CHAR_ON_FOOT_OBJECT_3D
    (1142, &[0, 0, 0, 0, 0, 0], 0),      // LOCATE_CHAR_IN_CAR_OBJECT_3D
    (1143, &[0, 0], 0),                  // SET_CAR_HANDBRAKE_TURN_LEFT
    (1144, &[0, 0], 0),                  // SET_CAR_HANDBRAKE_TURN_RIGHT
    (1145, &[0, 0], 0),                  // SET_CAR_HANDBRAKE_STOP
    (1146, &[0], 0),                     // IS_CHAR_ON_ANY_BIKE
    (1147, &[0, 0, 0, 0, 0], 0),         // LOCATE_SNIPER_BULLET_2D
    (1148, &[0, 0, 0, 0, 0, 0, 0], 0),   // LOCATE_SNIPER_BULLET_3D
    (1149, &[0], 1),                     // GET_NUMBER_OF_SEATS_IN_MODEL
    (1150, &[0], 0),                     // IS_PLAYER_ON_ANY_BIKE
    (1151, &[0], 0),                     // IS_CHAR_LYING_DOWN
    (1152, &[0, 0], 0),                  // CAN_CHAR_SEE_DEAD_CHAR
    (1153, &[0], 0),                     // SET_ENTER_CAR_RANGE_MULTIPLIER
    (1154, &[0], 0),                     // SET_THREAT_REACTION_RANGE_MULTIPLIER
];
