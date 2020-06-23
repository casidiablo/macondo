USER root

ARG MACONDO_HOST_USER_ID=MACONDO_HOST_USER_ID_PLACEHOLDER
ARG MACONDO_HOST_GROUP_ID=MACONDO_HOST_GROUP_ID_PLACEHOLDER
ARG MACONDO_HOST_USERNAME=MACONDO_HOST_USERNAME_PLACEHOLDER
ARG MACONDO_HOST_HOME_DIR=MACONDO_HOST_HOME_DIR_PLACEHOLDER

RUN if [ "${MACONDO_HOST_USER_ID:-0}" -ne 0 ] && [ "${MACONDO_HOST_GROUP_ID:-0}" -ne 0 ]; then \
        # If user already exists, delete it
        if getent passwd "${MACONDO_HOST_USERNAME}"; then \
            if type deluser; then \
                deluser "${MACONDO_HOST_USERNAME}"; \
            else \
                userdel -f "${MACONDO_HOST_USERNAME}"; \
            fi ;\
        fi &&\
        # If user already exists by id, delete it
        if getent passwd "${MACONDO_HOST_USER_ID}"; then \
            if type deluser; then \
                deluser "$(getent passwd "$MACONDO_HOST_USER_ID" | cut -d: -f1)"; \
            else \
                userdel -f "${MACONDO_HOST_USER_ID}"; \
            fi ;\
        fi &&\
        # When group ID already exists, reuse it
        if getent group "${MACONDO_HOST_GROUP_ID}" > /dev/null; then \
            group_name=$(getent group "${MACONDO_HOST_GROUP_ID}" | cut -d: -f1); \
        else \
        # when it does not, create a new group with a name that is likely not to conflict
            group_name="hard_to_duplicate_group_name" &&\
            addgroup -g "${MACONDO_HOST_GROUP_ID}" "$group_name" || addgroup -gid "${MACONDO_HOST_GROUP_ID}" "$group_name" || exit 1;\
        fi &&\
        # create user (try alpine syntax and default to ubuntu syntax)
        mkdir -p $MACONDO_HOST_HOME_DIR && chmod 755 $MACONDO_HOST_HOME_DIR && chown $MACONDO_HOST_USER_ID:$group_name $MACONDO_HOST_HOME_DIR &&\
        adduser --gecos "" --disabled-password --uid "$MACONDO_HOST_USER_ID" --ingroup "$group_name" --home "$MACONDO_HOST_HOME_DIR" "$MACONDO_HOST_USERNAME" --force-badname ||\
        adduser -D -u "$MACONDO_HOST_USER_ID" -G "$group_name" -h "$MACONDO_HOST_HOME_DIR" "$MACONDO_HOST_USERNAME" ||\
        exit 1 ;\
    fi

USER ${MACONDO_HOST_USERNAME}
